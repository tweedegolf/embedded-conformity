use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    process::exit,
    sync::{
        Arc,
        mpsc::{Receiver, Sender, TryRecvError, channel},
    },
    thread::{self, Thread, scope, sleep, yield_now},
    time::{Duration, Instant},
};

use parking_lot::FairMutex;
use probe_rs::{
    Session,
    rtt::{DownChannel, Rtt, UpChannel},
};
use serde::{Deserialize, Serialize};
use test_suite::{
    NUM_TESTS,
    postcard::accumulator::{CobsAccumulator, FeedResult},
    protocol::{
        DUTToHost, FPToHost, HostToDUT, HostToDUTCommand, HostToFP, HostToFPCommand, to_bytes_alloc,
    },
};
use tracing::{debug, error, info};

use crate::{
    Config,
    defmt_logger::{Target, run_logger},
};

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

    /// Initializes RTT and sets up the defmt logger
    fn init_channels(
        session: ArcSession,
        target: Target,
        elf: PathBuf,
    ) -> (UpChannel, DownChannel) {
        let mut rtt = {
            let mut guard = session.lock();
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

        thread::spawn(move || {
            run_logger(target, session, &mut defmt, elf);
        });

        (up_control, down_control)
    }

    /// Starts a thread that sends all data received on a channel to the device
    fn create_sender<T: Serialize + Send + 'static>(
        session: ArcSession,
        mut down: DownChannel,
    ) -> Sender<T> {
        let (tx, rx) = channel();
        thread::spawn(move || {
            loop {
                let data: T = rx.recv().unwrap();
                let mut guard = session.lock();
                let mut core = guard.core(0).unwrap();
                let buf = to_bytes_alloc(data);
                down.write(&mut core, &buf).unwrap();
            }
        });

        tx
    }

    fn create_receiver<T>(session: ArcSession, mut up: UpChannel) -> Receiver<T>
    where
        T: for<'de> Deserialize<'de> + Send + 'static,
    {
        let (tx, rx) = channel();
        thread::spawn(move || {
            let mut raw_buf = [0u8; 128];
            let mut cobs_buf: CobsAccumulator<256> = CobsAccumulator::new();

            while let Ok(ct) = {
                let mut guard = session.lock();
                let mut core = guard.core(0).unwrap();

                up.read(&mut core, &mut raw_buf)
            } {
                // Finished reading input
                if ct == 0 {
                    continue;
                }

                let buf = &raw_buf[..ct];
                let mut window = buf;

                'cobs: while !window.is_empty() {
                    window = match cobs_buf.feed::<T>(window) {
                        FeedResult::Consumed => break 'cobs,
                        FeedResult::OverFull(new_wind) => new_wind,
                        FeedResult::DeserError(new_wind) => new_wind,
                        FeedResult::Success { data, remaining } => {
                            tx.send(data).unwrap();
                            remaining
                        }
                    };
                }
            }
        });

        rx
    }

    pub fn run(&self) {
        let (fp_up, fp_down) = Self::init_channels(
            self.fp_session.clone(),
            Target::FakePeripheral,
            self.fp_elf.clone(),
        );
        let (dut_up, dut_down) = Self::init_channels(
            self.dut_session.clone(),
            Target::DeviceUnderTest,
            self.dut_elf.clone(),
        );

        // FP: Host to FP thread
        let to_fp = Self::create_sender(self.fp_session.clone(), fp_down);

        // DUT: Host to DUT thread
        let to_dut = Self::create_sender(self.dut_session.clone(), dut_down);

        // FP: FP to Host Thread
        let from_fp = Self::create_receiver(self.fp_session.clone(), fp_up);

        // DUT: DUT to Host Thread
        let from_dut = Self::create_receiver(self.dut_session.clone(), dut_up);

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

        let mut fp_acks = HashMap::new();
        let mut dut_acks = HashMap::new();

        info!("Devices initialized, starting tests...");

        for n in 0..NUM_TESTS {
            let fp_msg = HostToFP::new(HostToFPCommand::Run(n));
            let dut_msg = HostToDUT::new(HostToDUTCommand::Run(n));
            fp_acks.insert(fp_msg.id, Instant::now());
            dut_acks.insert(dut_msg.id, Instant::now());

            to_fp.send(fp_msg).unwrap();
            to_dut.send(dut_msg).unwrap();

            let mut fp_success = false;
            let mut dut_success = false;

            let now = Instant::now();

            'inner: loop {
                // Timeout, check for pending acks
                if check_timeouts(&fp_acks) {
                    error!(
                        "Fake Peripheral ack timeout({}ms) reached",
                        TIMEOUT.as_millis()
                    );
                    exit(1);
                }

                if check_timeouts(&dut_acks) {
                    error!(
                        "Device Under Test ack timeout({}ms) reached",
                        TIMEOUT.as_millis()
                    );
                    exit(1);
                }

                if now.elapsed() > TIMEOUT {
                    error!("Timeout, test took more than {}ms", TIMEOUT.as_millis());
                    exit(1);
                }

                match from_fp.try_recv() {
                    Ok(msg) => match msg {
                        FPToHost::Ack(id) => {
                            assert!(fp_acks.remove(&id).is_some());
                        }
                        FPToHost::TestFailure(n) => error!("FRM FP: Test {n} failed"),
                        FPToHost::Success(gn) => {
                            assert_eq!(n, gn);
                            debug!("fp success {n}");
                            fp_success = true;
                        }
                    },
                    Err(TryRecvError::Empty) => {}
                    Err(TryRecvError::Disconnected) => panic!("FP Disconnected"),
                }

                match from_dut.try_recv() {
                    Ok(msg) => match msg {
                        DUTToHost::Ack(id) => {
                            assert!(dut_acks.remove(&id).is_some());
                        }
                        DUTToHost::TestFailure(n) => error!("FRM DT: Test {n} failed"),
                        DUTToHost::Success(gn) => {
                            assert_eq!(n, gn);
                            debug!("dut success {n}");
                            dut_success = true;
                        }
                        DUTToHost::Finished => todo!(),
                    },
                    Err(TryRecvError::Empty) => {}
                    Err(TryRecvError::Disconnected) => panic!("FP Disconnected"),
                }

                if fp_success && dut_success {
                    break 'inner;
                }
            }

            info!("Test {n}: Success ({}ms)", now.elapsed().as_millis());
        }

        exit(0);
    }
}

pub const TIMEOUT: Duration = Duration::from_millis(5000);
fn check_timeouts(hm: &HashMap<u32, Instant>) -> bool {
    hm.iter().any(|(_, v)| v.elapsed() > TIMEOUT)
}
