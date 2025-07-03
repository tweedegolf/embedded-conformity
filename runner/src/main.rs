use std::error::Error;
use std::fs::{self, canonicalize};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use cargo::core::Workspace;
use cargo::core::compiler::CompileMode;
use cargo::ops::CompileOptions;
use cargo::util::interning::InternedString;
use cargo::{GlobalContext, ops};
use clap::{Parser, Subcommand};
use coordinator::Coordinator;
use probe_rs::config::TargetSelector;
use probe_rs::flashing::{
    DownloadOptions, Format, IdfOptions, download_file, download_file_with_options,
};
use probe_rs::probe::DebugProbeInfo;
use probe_rs::probe::list::Lister;
use probe_rs::{Permissions, Session};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

mod coordinator;
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
        #[arg(long = "config", short, default_value = "./config.toml")]
        config_file: PathBuf,
    },
    ExampleConfig,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Config {
    device_under_test: DeviceInfo,
    fake_peripheral: DeviceInfo,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct DeviceInfo {
    firmware_path: PathBuf,
    serial: String,
    chip: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    match cli.command {
        Commands::List => {
            let l = Lister::new();
            let probes = l.list_all();
            println!("{probes:#?}");
        }
        Commands::Test { config_file } => {
            let str = fs::read_to_string(config_file).unwrap();
            let cfg: Config = toml::from_str(&str).unwrap();
            run_test(cfg);
        }
        Commands::ExampleConfig => {
            let cfg = Config {
                device_under_test: DeviceInfo {
                    firmware_path: PathBuf::from_str("../nRF52/").unwrap(),
                    serial: "001050295885".to_owned(),
                    chip: "nRF52805_xxAA".to_owned(),
                },
                fake_peripheral: DeviceInfo {
                    firmware_path: PathBuf::from_str("../fake-peripheral/").unwrap(),
                    serial: "E6614103E78B5024".to_owned(),
                    chip: "rp2040".to_owned(),
                },
            };

            let res = toml::to_string_pretty(&cfg).unwrap();
            println!("{res}");
        }
    }

    Ok(())
}

fn run_test(cfg: Config) {
    let lister = Lister::new();
    let probes = lister.list_all();

    let fake_peripheral = probes
        .iter()
        .find(|el| el.serial_number.as_deref() == Some(&cfg.fake_peripheral.serial))
        .expect("Could not find fake_peripheral with uuid");

    let dut = probes
        .iter()
        .find(|el| el.serial_number.as_deref() == Some(&cfg.device_under_test.serial))
        .expect("Could not find dut with uuid");

    debug!("Devices found");

    let mut fake_path = cfg.fake_peripheral.firmware_path.clone();
    if !fake_path.ends_with("Cargo.toml") {
        fake_path.push("Cargo.toml");
    }

    let mut dut_path = cfg.device_under_test.firmware_path.clone();
    if !dut_path.ends_with("Cargo.toml") {
        dut_path.push("Cargo.toml");
    }

    let fake_elf = build_firmware(fake_path.as_path());
    let dut_elf = build_firmware(dut_path.as_path());
    debug!("Finished building fimrware");

    flash_firmware(
        fake_peripheral,
        &cfg.fake_peripheral.chip,
        fake_elf.as_path(),
    );
    debug!("Flashed FP");
    flash_firmware(dut, &cfg.device_under_test.chip, dut_elf.as_path());
    debug!("Flashed DUT");

    let dut_session = start_device(dut, &cfg.device_under_test.chip);
    debug!("Started DUT");
    let fp_session = start_device(fake_peripheral, &cfg.fake_peripheral.chip);
    debug!("Started FP");

    Coordinator::new(cfg, dut_session, dut_elf, fp_session, fake_elf).run();
}

#[tracing::instrument]
fn build_firmware(path: &Path) -> PathBuf {
    let mut gctx = GlobalContext::default().unwrap();
    // makes sure the correct `.cargo/config` is loaded
    gctx.reload_rooted_at(path).unwrap();

    let path = canonicalize(path).unwrap();
    let ws = Workspace::new(&path, &gctx).unwrap();
    let mut opts = CompileOptions::new(&gctx, CompileMode::Build).unwrap();

    opts.build_config.requested_profile = InternedString::new("dev");

    let mut comp = ops::compile(&ws, &opts).unwrap();
    assert!(comp.binaries.len() == 1);

    comp.binaries.pop().unwrap().path
}

#[tracing::instrument]
fn flash_firmware(
    probe_info: &DebugProbeInfo,
    target: impl Into<TargetSelector> + core::fmt::Debug,
    elf: &Path,
) {
    assert!(elf.exists(), "Elf path does not exist");

    let probe = probe_info.open().unwrap();

    let mut session = probe.attach(target, Permissions::default()).unwrap();

    let mut opts = DownloadOptions::default();
    opts.preverify = true;

    let format = if session
        .target()
        .default_format
        .as_ref()
        .is_some_and(|fmt| fmt == "idf")
    {
        debug!("Flashing IDF Bootloader");
        Format::Idf(IdfOptions::default())
    } else {
        Format::Elf
    };

    download_file_with_options(&mut session, elf, format, opts).unwrap();
}

#[tracing::instrument]
fn start_device(probe_info: &DebugProbeInfo, chip: &str) -> Session {
    let probe = probe_info.open().unwrap();
    let mut session = probe.attach(chip, Permissions::default()).unwrap();
    {
        let mut core = session.core(0).unwrap();
        core.reset().unwrap();
    }

    session
}
