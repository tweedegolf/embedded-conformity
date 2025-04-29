use std::path::Path;
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;

use probe_rs::Permissions;
use probe_rs::config::TargetSelector;
use probe_rs::flashing::{Format, download_file};
use probe_rs::probe::DebugProbeInfo;
use probe_rs::probe::list::Lister;
use probe_rs::rtt::Rtt;

// TODO: Use clap to take these as arguments
const DEBUG_PROBE_UUID: &str = "E6614103E78B5024";
const FAKE_PERIPHERAL_FIRMWARE_PATH: &str =
    "../fake-peripheral/target/thumbv6m-none-eabi/release/fake-peripheral";

// 1. Upload firmware to DUT: https://probe.rs/docs/library/quickstart/#downloading-to-flash
// 2. Upload firmware to client (RP2040)
// 3. Start Tests, https://docs.rs/rtt-target/latest/rtt_target/#reading
// 4. Report Status, use defmt and rtt to read back from the chips?

fn main() {
    let lister = Lister::new();
    let probes = lister.list_all();

    let probe_info = probes
        .iter()
        .find(|el| el.serial_number == Some(DEBUG_PROBE_UUID.to_owned()))
        .unwrap();

    build_firmware("../fake-peripheral/");
    flash_firmware(probe_info, "rp2040", FAKE_PERIPHERAL_FIRMWARE_PATH);

    start_fake_peripheral(probe_info);
}

fn build_firmware(path: impl AsRef<Path>) {
    Command::new("cargo")
        .args(["build", "--release"])
        .current_dir(path)
        .output()
        .unwrap();
}

fn flash_firmware(
    probe_info: &DebugProbeInfo,
    target: impl Into<TargetSelector>,
    elf: impl AsRef<Path>,
) {
    assert!(elf.as_ref().exists(), "Elf path does not exist");

    let probe = probe_info.open().unwrap();

    let mut session = probe.attach(target, Permissions::default()).unwrap();

    download_file(&mut session, elf, Format::Elf).unwrap();
}

fn start_fake_peripheral(probe_info: &DebugProbeInfo) {
    // TODO: What here is absolutely necessery
    let probe = probe_info.open().unwrap();
    let mut session = probe.attach("rp2040", Permissions::default()).unwrap();
    let mut core = session.core(0).unwrap();

    // TODO: Is this absolutely necessery?
    core.reset().unwrap();
    core.run().unwrap();

    let mut rtt = Rtt::attach(&mut core).unwrap();

    let down_channel = rtt.down_channel(0).unwrap();

    // Send "Start command"
    down_channel.write(&mut core, &[42]).unwrap();

    // Start reading from the client
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

fn start_dut(probe_info: &DebugProbeInfo) {
    todo!()
}
