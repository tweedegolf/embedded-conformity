use std::{env, fs, path::Path};

use defmt_decoder::{
    DecodeError, Table,
    log::format::{Formatter, FormatterConfig},
};
use probe_rs::{Core, rtt::UpChannel};

const READ_BUFFER_SIZE: usize = 1024;

pub fn run_logger(prefix: &str, core: &mut Core, up: &mut UpChannel, elf: impl AsRef<Path>) {
    // based on: https://github.com/knurling-rs/defmt/blob/66250db0584a8bf96323f2301f778f8f25d140a8/print/src/main.rs#L183

    // read and parse elf file
    let bytes = fs::read(elf).unwrap();
    let table = Table::parse(&bytes).unwrap().unwrap();
    let locs = table.get_locations(&bytes).unwrap();

    let locs = if table.indices().all(|idx| locs.contains_key(&(idx as u64))) {
        Some(locs)
    } else {
        // log::warn!("(BUG) location info is incomplete; it will be omitted from the output");
        None
    };

    let mut formatter_config = FormatterConfig::default();
    formatter_config.is_timestamp_available = table.has_timestamp();

    let formatter = Formatter::new(formatter_config);

    let mut buf = [0; READ_BUFFER_SIZE];
    let mut stream_decoder = table.new_stream_decoder();
    let current_dir = env::current_dir().unwrap();

    loop {
        let n = up.read(core, &mut buf).unwrap();

        stream_decoder.received(&buf[..n]);

        loop {
            match stream_decoder.decode() {
                Ok(frame) => {
                    println!("{prefix}, {}", frame.display(true));
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
