use std::{env, fs, path::Path};

use defmt_decoder::{DecodeError, Frame, Locations, Table};
use probe_rs::rtt::UpChannel;
use tracing::{Level, error, event, warn};

use crate::coordinator::ArcSession;
use defmt_parser::Level as DefmtLevel;

const READ_BUFFER_SIZE: usize = 1024;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Target {
    FakePeripheral,
    DeviceUnderTest,
}

pub fn run_logger(target: Target, session: ArcSession, up: &mut UpChannel, elf: impl AsRef<Path>) {
    // based on: https://github.com/knurling-rs/defmt/blob/66250db0584a8bf96323f2301f778f8f25d140a8/print/src/main.rs#L183

    // read and parse elf file
    let bytes = fs::read(elf).unwrap();
    let table = Table::parse(&bytes).unwrap().unwrap();
    let locs = table.get_locations(&bytes).unwrap();

    let locs = if table.indices().all(|idx| locs.contains_key(&(idx as u64))) {
        Some(locs)
    } else {
        warn!("location info is incomplete; it will be omitted from the output");
        None
    };

    let mut buf = [0; READ_BUFFER_SIZE];
    let mut stream_decoder = table.new_stream_decoder();
    let current_dir = env::current_dir().unwrap();

    loop {
        let n = {
            let mut guard = session.lock();
            let mut core = guard.core(0).unwrap();
            up.read(&mut core, &mut buf).unwrap()
        };

        stream_decoder.received(&buf[..n]);

        loop {
            match stream_decoder.decode() {
                Ok(frame) => {
                    let location_info = location_info(locs.as_ref(), &frame, &current_dir);
                    log_frame(target, &frame, &location_info);
                }
                Err(DecodeError::UnexpectedEof) => break,
                Err(DecodeError::Malformed) => match table.encoding().can_recover() {
                    false => panic!("malformed defmt, unrecoverable from {:?}", target),
                    true => {
                        // log error
                        error!("malformed frame from {:?}", target);
                        continue;
                    }
                },
            }
        }
    }
}

type LocationInfo = (Option<String>, Option<u32>, Option<String>);

fn location_info(locs: Option<&Locations>, frame: &Frame, current_dir: &Path) -> LocationInfo {
    let (mut file, mut line, mut mod_path) = (None, None, None);

    let loc = locs.as_ref().map(|locs| locs.get(&frame.index()));

    if let Some(Some(loc)) = loc {
        // try to get the relative path, else the full one
        let path = loc.file.strip_prefix(current_dir).unwrap_or(&loc.file);

        file = Some(path.display().to_string());
        line = Some(loc.line as u32);
        mod_path = Some(loc.module.clone());
    }

    (file, line, mod_path)
}

macro_rules! log_event {
    ($target:expr, $level:expr, $loc:expr, $frame:expr) => {
        event!(
            target: $target,
            $level,
            file = $loc.0,
            line = $loc.1,
            module = $loc.2,
            "{}",
            $frame.display_message()
        )
    };
}

fn log_frame(target: Target, frame: &Frame<'_>, loc: &LocationInfo) {
    const FP_TARGET: &str = "fp";
    const DUT_TARGET: &str = "dut";

    match target {
        Target::FakePeripheral => match frame.level() {
            Some(DefmtLevel::Trace) => log_event!(FP_TARGET, Level::TRACE, loc, frame),
            Some(DefmtLevel::Debug) => log_event!(FP_TARGET, Level::DEBUG, loc, frame),
            Some(DefmtLevel::Info) => log_event!(FP_TARGET, Level::INFO, loc, frame),
            None | Some(DefmtLevel::Warn) => log_event!(FP_TARGET, Level::WARN, loc, frame),
            Some(DefmtLevel::Error) => log_event!(FP_TARGET, Level::ERROR, loc, frame),
        },
        Target::DeviceUnderTest => match frame.level() {
            Some(DefmtLevel::Trace) => log_event!(DUT_TARGET, Level::TRACE, loc, frame),
            Some(DefmtLevel::Debug) => log_event!(DUT_TARGET, Level::DEBUG, loc, frame),
            Some(DefmtLevel::Info) => log_event!(DUT_TARGET, Level::INFO, loc, frame),
            None | Some(DefmtLevel::Warn) => log_event!(DUT_TARGET, Level::WARN, loc, frame),
            Some(DefmtLevel::Error) => log_event!(DUT_TARGET, Level::ERROR, loc, frame),
        },
    }
}
