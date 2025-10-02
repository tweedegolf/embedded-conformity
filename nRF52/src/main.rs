#![no_std]
#![no_main]

use cortex_m::asm::wfi;
use embassy_executor::Spawner;
use embassy_nrf::{
    bind_interrupts,
    gpio::{Level, Output, OutputDrive},
    peripherals,
    twim::{self, Twim},
};
use panic_probe as _;

use test_suite::dut::{DutPeripherals, run_dut_tests};

bind_interrupts!(struct Irqs {
    TWISPI0 => twim::InterruptHandler<peripherals::TWISPI0>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let ctx = test_suite::init();
    let p = embassy_nrf::init(Default::default());

    let output_a = Output::new(p.P0_31, Level::Low, OutputDrive::Standard);

    let config = twim::Config::default();
    let twim = Twim::new(p.TWISPI0, Irqs, p.P0_03, p.P0_04, config);

    let session = DutPeripherals {
        i2c: twim,
        pin: output_a,
    };

    run_dut_tests(ctx, session);

    loop {
        wfi();
    }
}
