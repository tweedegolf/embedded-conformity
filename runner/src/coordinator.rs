use std::{
    path::PathBuf,
    sync::{Arc, mpsc::channel},
    thread::{self, Thread, scope},
};

use parking_lot::FairMutex;
use probe_rs::{Session, rtt::Rtt};
use test_suite::{
    postcard::accumulator::{CobsAccumulator, FeedResult},
    protocol::{
        DUTToHost, FPToHost, HostToDUT, HostToDUTCommand, HostToFP, HostToFPCommand, to_bytes_alloc,
    },
};

use crate::{Config, defmt_logger::run_logger};

pub type ArcSession = Arc<FairMutex<Session>>;

pub struct Coordinator {
    dut_session: ArcSession,
    dut_elf: PathBuf,

    fp_session: ArcSession,
    fp_elf: PathBuf,

    config: Config,
}

impl Coordinator {
    pub fn new(
        cfg: Config,
        dut_session: Session,
        dut_elf: PathBuf,
        fp_session: Session,
        fp_elf: PathBuf,
    ) -> Self {
        Coordinator {
            dut_session: Arc::new(FairMutex::new(dut_session)),
            fp_session: Arc::new(FairMutex::new(fp_session)),
            config: cfg,
            dut_elf,
            fp_elf,
        }
    }

    pub fn run(&self) {
        scope(|s| {
            let (mut fp_up, mut fp_down) = {
                let mut rtt = {
                    let mut guard = self.fp_session.lock();
                    let mut core = guard.core(0).unwrap();

                    match Rtt::attach(&mut core) {
                        Ok(rtt) => rtt,
                        // Workaround for nRF52840_xxAA
                        // https://github.com/probe-rs/probe-rs/issues/2242
                        Err(probe_rs::rtt::Error::MultipleControlBlocksFound(mut rtts)) => {
                            rtts.pop().unwrap()
                        }
                        e @ Err(_) => e.unwrap(),
                    }
                };

                let up_control = rtt.up_channels.pop().unwrap();
                let down_control = rtt.down_channels.pop().unwrap();
                let mut defmt = rtt.up_channels.pop().unwrap();

                let fp_session = self.fp_session.clone();
                s.spawn(move || {
                    run_logger(" fp", fp_session, &mut defmt, self.fp_elf.as_path());
                });

                (up_control, down_control)
            };

            let (mut dut_up, mut dut_down) = {
                let mut rtt = {
                    let mut guard = self.dut_session.lock();
                    let mut core = guard.core(0).unwrap();

                    match Rtt::attach(&mut core) {
                        Ok(rtt) => rtt,
                        // Workaround for nRF52840_xxAA
                        // https://github.com/probe-rs/probe-rs/issues/2242
                        Err(probe_rs::rtt::Error::MultipleControlBlocksFound(mut rtts)) => {
                            rtts.pop().unwrap()
                        }
                        e @ Err(_) => e.unwrap(),
                    }
                };

                let up_control = rtt.up_channels.pop().unwrap();
                let down_control = rtt.down_channels.pop().unwrap();
                let mut defmt = rtt.up_channels.pop().unwrap();

                let dut_session = self.dut_session.clone();
                s.spawn(move || {
                    run_logger("dut", dut_session, &mut defmt, self.dut_elf.as_path());
                });

                (up_control, down_control)
            };

            // FP: Host to FP thread
            let fp_session = self.fp_session.clone();
            let (to_fp, to_fp_rx) = channel();
            s.spawn(move || {
                loop {
                    let data: HostToFP = to_fp_rx.recv().unwrap();
                    let mut guard = fp_session.lock();
                    let mut core = guard.core(0).unwrap();
                    let mut buf = to_bytes_alloc(data);
                    fp_down.write(&mut core, &mut buf).unwrap();

                    drop(core);
                    drop(guard);
                }
            });

            // DUT: Host to DUT thread
            let dut_session = self.dut_session.clone();
            let (to_dut, to_dut_rx) = channel();
            s.spawn(move || {
                loop {
                    let data: HostToDUT = to_dut_rx.recv().unwrap();
                    let mut guard = dut_session.lock();
                    let mut core = guard.core(0).unwrap();
                    let mut buf = to_bytes_alloc(data);
                    dut_down.write(&mut core, &mut buf).unwrap();

                    drop(core);
                    drop(guard);
                }
            });

            // FP: FP to Host Thread
            let fp_session = self.fp_session.clone();
            let (from_fp_tx, from_fp) = channel();
            s.spawn(move || {
                let mut raw_buf = [0u8; 128];
                let mut cobs_buf: CobsAccumulator<256> = CobsAccumulator::new();

                while let Ok(ct) = {
                    let mut guard = fp_session.lock();
                    let mut core = guard.core(0).unwrap();

                    fp_up.read(&mut core, &mut raw_buf)
                } {
                    // Finished reading input
                    if ct == 0 {
                        continue;
                    }

                    let buf = &raw_buf[..ct];
                    let mut window = &buf[..];

                    'cobs: while !window.is_empty() {
                        window = match cobs_buf.feed::<FPToHost>(&window) {
                            FeedResult::Consumed => break 'cobs,
                            FeedResult::OverFull(new_wind) => new_wind,
                            FeedResult::DeserError(new_wind) => new_wind,
                            FeedResult::Success { data, remaining } => {
                                from_fp_tx.send(data).unwrap();
                                remaining
                            }
                        };
                    }
                }
            });

            // DUT: DUT to Host Thread
            let dut_session = self.dut_session.clone();
            let (from_dut_tx, from_dut) = channel();
            s.spawn(move || {
                let mut raw_buf = [0u8; 128];
                let mut cobs_buf: CobsAccumulator<256> = CobsAccumulator::new();

                while let Ok(ct) = {
                    let mut guard = dut_session.lock();
                    let mut core = guard.core(0).unwrap();

                    dut_up.read(&mut core, &mut raw_buf)
                } {
                    // Finished reading input
                    if ct == 0 {
                        continue;
                    }

                    let buf = &raw_buf[..ct];
                    let mut window = &buf[..];

                    'cobs: while !window.is_empty() {
                        window = match cobs_buf.feed::<DUTToHost>(&window) {
                            FeedResult::Consumed => break 'cobs,
                            FeedResult::OverFull(new_wind) => new_wind,
                            FeedResult::DeserError(new_wind) => new_wind,
                            FeedResult::Success { data, remaining } => {
                                from_dut_tx.send(data).unwrap();

                                remaining
                            }
                        };
                    }
                }
            });

            println!("set up threads, sending init ...");

            to_fp
                .send(HostToFP {
                    id: 13,
                    command: HostToFPCommand::Init,
                })
                .unwrap();
            to_dut
                .send(HostToDUT {
                    id: 31,
                    command: HostToDUTCommand::Init,
                })
                .unwrap();

            assert_eq!(FPToHost::Ack(13), from_fp.recv().unwrap());
            assert_eq!(DUTToHost::Ack(31), from_dut.recv().unwrap());

            println!("Acks Received!");

            loop {
                thread::yield_now();
            }
        });
    }
}
