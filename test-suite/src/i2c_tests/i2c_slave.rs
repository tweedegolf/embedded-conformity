#![cfg(feature = "fp")]

use embassy_rp::config;
use embassy_rp::gpio::Pull;
use embassy_rp::pio::{Direction, ShiftConfig, ShiftDirection};
use embassy_rp::pio::{Config, Pio, program::pio_file};
use defmt::{debug, trace, unimplemented};
use serde::de;

pub async fn test_pio_i2c_slave<'a, I: embassy_rp::pio::Instance>(
    pio: &mut Pio<'a, I>,
    sda: &mut embassy_rp::pio::Pin<'a, I>,
    scl: &mut embassy_rp::pio::Pin<'a, I>,
) {
    debug!("Configuring PIO for I2C slave");
    let program = pio_file!("src/i2c_tests/i2c_slave.pio", select_program("i2c_slave"));
    let program = pio.common.load_program(&program.program);
    
    sda.set_pull(Pull::Up);
    scl.set_pull(Pull::Up);

    let mut config = Config::<I>::default();
    config.set_in_pins(&[sda]);
    config.use_program(&program, &[scl]);

    pio.sm0.set_config(&config);
    pio.sm0.set_pin_dirs(Direction::In, &[sda, scl]);

    debug!("Running PIO I2C slave program");
    pio.sm0.set_enable(true); // Start the state machine
    
    pio.irq0.wait().await; // Wait for the IRQ to be triggered
    pio.irq_flags.clear(0);
    debug!("I2C: start recieved");

    pio.irq1.wait().await; // Wait for the IRQ to be triggered
    let rx = pio.sm0.rx();
    debug!("I2C: address received: {:X}", unsafe { pio.sm0.get_x() });

    unimplemented!("Handle I2C slave communication here");
}
