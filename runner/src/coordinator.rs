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

use object::{Object, ObjectSymbol};
use parking_lot::FairMutex;
use probe_rs::{
    Session,
    rtt::{DownChannel, Rtt, ScanRegion, UpChannel},
};
use serde::{Deserialize, Serialize};
use test_suite::{
    list_of_tests::TestSelector,
    postcard::accumulator::{CobsAccumulator, FeedResult},
    protocol::{
        DUTToHost, FPToHost, HostToDUT, HostToDUTCommand, HostToFP, HostToFPCommand, to_bytes_alloc,
    },
    strum::IntoEnumIterator as _,
};
use tracing::{debug, error, info, warn};

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
        let elf_file = std::fs::read(&elf).unwrap();
        let elf_file = object::File::parse(&*elf_file).unwrap();
        let rtt_addr = elf_file.symbol_by_name("_SEGGER_RTT").map(|s| s.address());

        debug!("initing channels for {:?}", target);
        let mut rtt = {
            let mut guard = session.lock();
            let mut core = guard.core(0).unwrap();

            if let Some(addr) = rtt_addr {
                Rtt::attach_at(&mut core, addr).unwrap()
            } else {
                match Rtt::attach(&mut core) {
                    Ok(rtt) => rtt,
                    // Workaround for nRF52840_xxAA
                    // https://github.com/probe-rs/probe-rs/issues/2242
                    Err(probe_rs::rtt::Error::MultipleControlBlocksFound(rtts)) => {
                        Rtt::attach_at(&mut core, rtts[0]).unwrap()
                    }
                    e @ Err(_) => e.unwrap(),
                }
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

    pub fn run(self, selector: Option<TestSelector>) {
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

        // Create threads for bidirectional communication between HOST <-> {fp, dut}
        let to_fp = Self::create_sender(self.fp_session.clone(), fp_down);
        let to_dut = Self::create_sender(self.dut_session.clone(), dut_down);
        let from_fp = Self::create_receiver(self.fp_session.clone(), fp_up);
        let from_dut = Self::create_receiver(self.dut_session.clone(), dut_up);

        let init_fp = HostToFP::new(HostToFPCommand::Init);
        to_fp.send(init_fp).unwrap();
        let init_dut = HostToDUT::new(HostToDUTCommand::Init);
        to_dut.send(init_dut).unwrap();

        assert_eq!(FPToHost::Ack(init_fp.id), from_fp.recv().unwrap());
        assert_eq!(DUTToHost::Ack(init_dut.id), from_dut.recv().unwrap());

        let mut fp_acks = HashMap::new();
        let mut dut_acks = HashMap::new();

        info!("Devices initialized, starting tests...");

        let tests = selector
            .map(|t| vec![t])
            .unwrap_or(TestSelector::iter().collect());

        for t in tests {
            let fp_msg = HostToFP::new(HostToFPCommand::Run(t));
            let dut_msg = HostToDUT::new(HostToDUTCommand::Run(t));
            fp_acks.insert(fp_msg.id, Instant::now());
            dut_acks.insert(dut_msg.id, Instant::now());

            to_fp.send(fp_msg).unwrap();
            sleep(Duration::from_millis(500));
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
                    error!("Timeout: {t:?} took more than {}ms", TIMEOUT.as_millis());
                    exit(1);
                }

                match from_fp.try_recv() {
                    Ok(msg) => match msg {
                        FPToHost::Ack(id) => {
                            assert!(fp_acks.remove(&id).is_some());
                        }
                        FPToHost::TestFailure(a) => error!("FP: Test {a:?} failed"),
                        FPToHost::Success(a) => {
                            assert_eq!(t, a);
                            debug!("fp success {t:?}");
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
                        DUTToHost::TestFailure(a, msg) => {
                            error!("DT: Test {a:?} failed: {msg}");
                            dut_success = true;
                        }
                        DUTToHost::PartialSuccess(a, msg) => {
                            warn!("DT: Test {a:?} partially succeeded: {msg}");
                            dut_success = true;
                        }
                        DUTToHost::Success(a) => {
                            assert_eq!(t, a);
                            debug!("dut success {t:?}");
                            dut_success = true;
                        }
                    },
                    Err(TryRecvError::Empty) => {}
                    Err(TryRecvError::Disconnected) => panic!("DUT Disconnected"),
                }

                if fp_success && dut_success {
                    break 'inner;
                }
            }

            info!("Test {t:?}: Success ({}ms)", now.elapsed().as_millis());
        }

        exit(0);
    }
}

pub const TIMEOUT: Duration = Duration::from_millis(10_000);
fn check_timeouts(hm: &HashMap<u32, Instant>) -> bool {
    hm.iter().any(|(_, v)| v.elapsed() > TIMEOUT)
}
