#![cfg(feature = "fp")]

use defmt::{assert_eq, debug, intern, unimplemented};
use embassy_rp::gpio::Pull;
use embassy_rp::pio::{Config, Pio, program::pio_file};
use embassy_rp::pio::{Direction, ShiftConfig, ShiftDirection};

pub async fn test_pio_i2c_slave<'a, I: embassy_rp::pio::Instance>(
    pio: &mut Pio<'a, I>,
    sda: &mut embassy_rp::pio::Pin<'a, I>,
    scl: &mut embassy_rp::pio::Pin<'a, I>,
) {
    debug!("Configuring PIO for I2C slave");
    let program = pio_file!("src/i2c_tests/i2c_slave.pio", select_program("i2c_slave"));
    let program = pio.common.load_program(&program.program);

    // i2c requires pull-up
    sda.set_pull(Pull::Up);
    scl.set_pull(Pull::Up);

    let mut config = Config::<I>::default();
    config.set_in_pins(&[sda, scl]);
    config.set_out_pins(&[sda]);
    config.set_set_pins(&[sda]);
    config.use_program(&program, &[sda]);
    config.shift_in = ShiftConfig {
        threshold: 8,
        direction: ShiftDirection::Left,
        auto_fill: true,
    };

    pio.sm0.set_config(&config);
    pio.sm0.set_pin_dirs(Direction::In, &[sda, scl]);

    pio.sm0.set_enable(true); // Start the state machine

    pio.irq0.wait().await; // Wait for the IRQ to be triggered

    pio.irq1.wait().await; // Wait for the IRQ to be triggered
    let rx = pio.sm0.rx();
    let data = rx.pull().to_be_bytes()[3];

    let address = data >> 1;
    let mode = data & 1 == 1; // true is read, false is write

    debug!(
        "I2C Start, Address: 0x{:X}, Mode: {};",
        address,
        if mode {
            intern!("Read")
        } else {
            intern!("Write")
        }
    );

    pio.irq2.wait().await;

    debug!("I2C address acked");

    unimplemented!("Handle I2C slave communication here");
}
