use std::path::Path;
use std::thread::sleep;
use std::time::Duration;

use probe_rs::Permissions;
use probe_rs::flashing::{
    DownloadOptions, FlashProgress, Format, download_file, download_file_with_options,
};
use probe_rs::probe::{Probe, list::Lister};
use probe_rs::rtt::Rtt;
use probe_rs::rtt::ScanRegion::Ram;

// TODO: Use clap to take these as arguments
const DEBUG_PROBE_UUID: &str = "E6614103E78B5024";
const FAKE_PERIPHERAL_FIRMWARE_PATH: &str =
    "../fake-peripheral/target/thumbv6m-none-eabi/release/fake-peripheral";

fn main() {
    let lister = Lister::new();
    let probes = lister.list_all();

    // 1. Upload firmware to DUT: https://probe.rs/docs/library/quickstart/#downloading-to-flash
    // 2. Upload firmware to client (RP2040)
    // 3. Start Tests, https://docs.rs/rtt-target/latest/rtt_target/#reading ?
    // 4. Report Status, use defmt and rtt to read back from the chips?
    //

    let probe_info = probes
        .iter()
        .find(|el| el.serial_number == Some(DEBUG_PROBE_UUID.to_owned()))
        .unwrap();

    dbg!(&probe_info);

    {
        let probe = probe_info.open().unwrap();
        let mut session = probe.attach("rp2040", Permissions::default()).unwrap();

        let progress = FlashProgress::new(|event| println!("Event: {:#?}", event));
        let mut options = DownloadOptions::new();
        options.progress = Some(progress);

        assert!(Path::new(FAKE_PERIPHERAL_FIRMWARE_PATH).exists());

        download_file_with_options(
            &mut session,
            FAKE_PERIPHERAL_FIRMWARE_PATH,
            Format::Elf,
            options,
        )
        .unwrap();
    }

    let probe = probe_info.open().unwrap();
    let mut session = probe.attach("rp2040", Permissions::default()).unwrap();
    let mut core = session.core(0).unwrap();

    // TODO: Is this absolutely necessery?
    core.reset().unwrap();
    core.run().unwrap();

    let mut rtt = Rtt::attach(&mut core).unwrap();

    let up_channel = rtt.up_channel(0).unwrap();

    loop {
        let mut buffer = [0u8; 1024];
        match up_channel.read(&mut core, &mut buffer) {
            Ok(n) if n > 0 => {
                let s = String::from_utf8_lossy(&buffer[..n]);
                print!("{}", s);
            }
            _ => {
                sleep(Duration::from_millis(50));
            }
        }
    }
}

fn upload_firmware_peripheral() {}
