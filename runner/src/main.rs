use std::fs::canonicalize;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread::{self, Thread, sleep};
use std::time::Duration;

use cargo::core::Workspace;
use cargo::core::compiler::CompileMode;
use cargo::ops::CompileOptions;
use cargo::util::interning::InternedString;
use cargo::{GlobalContext, ops};
use clap::{Parser, Subcommand};
use defmt_logger::run_logger;
use probe_rs::Permissions;
use probe_rs::config::TargetSelector;
use probe_rs::flashing::{Format, download_file};
use probe_rs::probe::DebugProbeInfo;
use probe_rs::probe::list::Lister;
use probe_rs::rtt::Rtt;

mod defmt_logger;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    List,
    Test {
        #[arg(long)]
        fake_peripheral_uuid: String,
        #[arg(long)]
        device_under_test_uuid: String,
    },
}

// 1. Upload firmware to DUT: https://probe.rs/docs/library/quickstart/#downloading-to-flash
// 2. Upload firmware to client (RP2040)
// 3. Start Tests, https://docs.rs/rtt-target/latest/rtt_target/#reading
// 4. Report Status, use defmt and rtt to read back from the chips?

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::List => {
            let l = Lister::new();
            let probes = l.list_all();
            println!("{probes:#?}");
        }
        Commands::Test {
            fake_peripheral_uuid,
            device_under_test_uuid,
        } => {
            run_test(&fake_peripheral_uuid, &device_under_test_uuid);
        }
    }
}

fn run_test(fake: &str, dut: &str) {
    let lister = Lister::new();
    let probes = lister.list_all();

    let fake_peripheral = probes
        .iter()
        .find(|el| el.serial_number.as_deref() == Some(fake))
        .expect("Could not find fake_peripheral with uuid");

    let dut = probes
        .iter()
        .find(|el| el.serial_number.as_deref() == Some(dut))
        .expect("Could not find dut with uuid");

    // TODO Remove these hardcoded values
    let fake_elf = build_firmware("../fake-peripheral/Cargo.toml");
    let dut_elf = build_firmware("../nRF52/Cargo.toml");

    flash_firmware(fake_peripheral, "rp2040", fake_elf.as_path());
    flash_firmware(dut, "nRF52840_xxAA", dut_elf.as_path());

    thread::scope(|s| {
        s.spawn(|| {
            start_dut(dut, dut_elf.as_path());
        });

        s.spawn(|| {
            start_fake_peripheral(fake_peripheral, fake_elf.as_path());
        });
    });
}

fn build_firmware(path: impl AsRef<Path>) -> PathBuf {
    let mut gctx = GlobalContext::default().unwrap();
    // makes sure the correct `.cargo/config` is loaded
    gctx.reload_rooted_at(&path).unwrap();

    let path = canonicalize(path).unwrap();
    let ws = Workspace::new(&path, &gctx).unwrap();
    let mut opts = CompileOptions::new(&gctx, CompileMode::Build).unwrap();

    opts.build_config.requested_profile = InternedString::new("release");

    let mut comp = ops::compile(&ws, &opts).unwrap();
    assert!(comp.binaries.len() == 1);

    comp.binaries.pop().unwrap().path
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

fn start_fake_peripheral(probe_info: &DebugProbeInfo, elf: impl AsRef<Path>) {
    // todo: what here is absolutely necessary
    let probe = probe_info.open().unwrap();
    let mut session = probe.attach("rp2040", Permissions::default()).unwrap();
    let mut core = session.core(0).unwrap();

    // TODO: Is this absolutely necessary?
    core.reset().unwrap();
    // core.run().unwrap();

    let mut rtt = Rtt::attach(&mut core).unwrap();

    let down_channel = rtt.down_channel(0).unwrap();

    // Send "Start command"
    down_channel.write(&mut core, &[42]).unwrap();

    // Start reading from the client
    let mut up_channel = rtt.up_channel(0).unwrap();
    run_logger("fp", &mut core, &mut up_channel, elf);
}

fn start_dut(probe_info: &DebugProbeInfo, elf: impl AsRef<Path>) {
    // TODO: What here is absolutely necessary
    let probe = probe_info.open().unwrap();
    let mut session = probe
        .attach("nRF52840_xxAA", Permissions::default())
        .unwrap();
    let mut core = session.core(0).unwrap();

    // TODO: Is this absolutely necessary?
    core.reset().unwrap();

    let mut rtt = match Rtt::attach(&mut core) {
        Ok(rtt) => rtt,
        // Workaround for nRF52840_xxAA
        // https://github.com/probe-rs/probe-rs/issues/2242
        Err(probe_rs::rtt::Error::MultipleControlBlocksFound(mut rtts)) => rtts.pop().unwrap(),
        e @ Err(_) => e.unwrap(),
    };

    let down_channel = rtt.down_channel(0).unwrap();

    // Send "Start command"
    down_channel
        .write(&mut core, &[test_suite::MAGIC_START_BYTE])
        .unwrap();

    // Start reading from the client
    let mut up_channel = rtt.up_channel(0).unwrap();
    run_logger("dut", &mut core, &mut up_channel, elf);
}
