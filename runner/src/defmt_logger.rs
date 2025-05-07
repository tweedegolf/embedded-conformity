use std::{env, fs, path::Path};

use defmt_decoder::{
    DecodeError, Frame, Locations, Table,
    log::format::{Formatter, FormatterConfig},
};
use log::Record;
use probe_rs::{Core, rtt::UpChannel};
use tracing::warn;
use tracing_log::AsTrace;

use crate::coordinator::ArcSession;

const READ_BUFFER_SIZE: usize = 1024;

pub fn run_logger(prefix: &str, session: ArcSession, up: &mut UpChannel, elf: impl AsRef<Path>) {
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

    // let mut formatter_config = FormatterConfig::default();
    // formatter_config.is_timestamp_available = table.has_timestamp();

    // let formatter = Formatter::new(formatter_config);

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
                    let location_info = location_info(&locs, &frame, &current_dir);
                    log_frame(prefix, frame, location_info);
                }
                Err(DecodeError::UnexpectedEof) => break,
                Err(DecodeError::Malformed) => match table.encoding().can_recover() {
                    false => panic!("malformed defmt, unrecoverable"),
                    true => {
                        // log error
                        eprintln!("{prefix} malformed frame");
                        continue;
                    }
                },
            }
        }
    }
}

type LocationInfo = (Option<String>, Option<u32>, Option<String>);

fn location_info(locs: &Option<Locations>, frame: &Frame, current_dir: &Path) -> LocationInfo {
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

fn log_frame(target: &str, frame: Frame<'_>, loc: LocationInfo) {
    let level = match frame.level() {
        Some(defmt_parser::Level::Trace) => log::Level::Trace,
        Some(defmt_parser::Level::Debug) => log::Level::Debug,
        Some(defmt_parser::Level::Info) => log::Level::Info,
        None | Some(defmt_parser::Level::Warn) => log::Level::Warn,
        Some(defmt_parser::Level::Error) => log::Level::Error,
    };

    log::logger().log(&Record::builder()
        .args(format_args!("{}", frame.display_message()))
        .target(target)
        .level(level)
        .module_path(loc.2.as_deref())
        .file(loc.0.as_deref())
        .line(loc.1)
        .build());
}
