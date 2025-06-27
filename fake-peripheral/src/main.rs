#![no_std]
#![no_main]

use embassy_executor::Spawner;
use panic_probe as _;
use test_suite::fp::{FPPeripherals, PioPeripheral, embassy_rp, run_fp_tests};
use test_suite::{
    fp::embassy_rp::{
        bind_interrupts,
        gpio::{Input, Level, Output},
        i2c,
        i2c_slave::{self, I2cSlave},
        peripherals::{I2C0, PIO0},
        pio::{self, Pio},
    },
    heapless::Vec,
    i2c_tests::I2C_DEFAULT_ADDRESS,
};

bind_interrupts!(struct I2cIrq {
    I2C0_IRQ => i2c::InterruptHandler<I2C0>;
});

bind_interrupts!(struct PioIrq {
    PIO0_IRQ_0 => pio::InterruptHandler<PIO0>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let ctx = test_suite::init();
    let p = embassy_rp::init(Default::default());

    let mut led = Output::new(p.PIN_13, Level::Low);
    led.set_high();

    let input_one = Input::new(p.PIN_10, embassy_rp::gpio::Pull::None);

    let mut config = i2c_slave::Config::default();
    config.addr = I2C_DEFAULT_ADDRESS as u16;
    // scl, sda
    let slave = I2cSlave::new(p.I2C0, p.PIN_1, p.PIN_0, I2cIrq, config);

    let scl = p.PIN_9;
    let sda = p.PIN_8;

    let mut pio = Pio::new(p.PIO0, PioIrq);
    let scl = pio.common.make_pio_pin(scl);
    let sda = pio.common.make_pio_pin(sda);

    let peripherals = FPPeripherals {
        i2c: slave,
        pio: PioPeripheral {
            pio,
            scl,
            sda,
            programs: Vec::new(),
        },

        pin: input_one,
    };

    run_fp_tests(ctx, peripherals).await;
}
