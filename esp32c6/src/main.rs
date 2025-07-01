#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

use defmt::info;
use rtt_target::{self, rtt_init, rtt_init_defmt, set_defmt_channel, ChannelMode};
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_hal::clock::CpuClock;
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp_hal::i2c::master::{Config as I2cConfig, I2c};
use esp_hal::peripherals::LP_I2C0;
use esp_hal::riscv::asm::wfi;
use esp_hal::timer::systimer::SystemTimer;
use panic_rtt_target as _;
use test_suite::dut::{run_dut_tests, DutPeripherals};
use test_suite::{Channels, Context};

// This creates a default app-descriptor required by the esp-idf bootloader.
esp_bootloader_esp_idf::esp_app_desc!();

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    let ctx = test_suite::init();
    info!("Hello world!");

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let p = esp_hal::init(config);

    let pin = Output::new(p.GPIO0, Level::Low, OutputConfig::default());

    let i2c = I2c::new(p.I2C0, I2cConfig::default())
        .unwrap()
        .with_scl(p.GPIO8)
        .with_sda(p.GPIO9);

    let session = DutPeripherals { i2c, pin };

    run_dut_tests(ctx, session);

    loop {
        wfi();
    }
}
