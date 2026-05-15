#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_stm32::{
    bind_interrupts,
    gpio::{Level, Output, Speed},
    i2c::{self, I2c},
    peripherals,
};
use panic_probe as _;
use test_suite::dut::{run_dut_tests, DutPeripherals};

bind_interrupts!(
    /// Binds the I2C interrupts.
    struct Irqs {
        I2C1_EV => i2c::EventInterruptHandler<peripherals::I2C1>;
        I2C1_ER => i2c::ErrorInterruptHandler<peripherals::I2C1>;
    }
);

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let ctx = test_suite::init();
    let p = embassy_stm32::init(Default::default());

    let pin = Output::new(p.PA6, Level::Low, Speed::Low);

    let config = i2c::Config::default();

    let i2c = I2c::new_blocking(p.I2C1, p.PB8, p.PB9, config);

    let session = DutPeripherals { i2c, pin };

    run_dut_tests(ctx, session);
}
