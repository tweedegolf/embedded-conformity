#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp::{bind_interrupts, gpio::{Input, Level, Output}, i2c::InterruptHandler, i2c_slave::{self, I2cSlave}, peripherals::{I2C0, I2C1, PIO0}};
use embassy_time::{Duration, with_timeout};
use panic_probe as _;
use test_suite::{embassy_rp, i2c_tests::I2C_DEFAULT_ADDRESS};

// mod pio;
bind_interrupts!(struct Irqs {
    I2C0_IRQ => InterruptHandler<I2C0>;
});



#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let ctx = test_suite::init();
    let p = embassy_rp::init(Default::default());

    let mut led = Output::new(p.PIN_13, Level::Low);
    led.set_high();

    let mut input_one = Input::new(p.PIN_10, embassy_rp::gpio::Pull::None);

    let config = i2c_slave::Config::default();
    // scl, sda
    let mut slave = I2cSlave::new(p.I2C0, p.PIN_9, p.PIN_8, Irqs, config);

    test_suite::run_fp_tests(ctx, &mut input_one, &mut slave).await;
}

