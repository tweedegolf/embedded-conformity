#![cfg(feature = "fp")]

use embassy_rp::config;
use embassy_rp::gpio::Pull;
use embassy_rp::pio::Direction;
use embassy_rp::pio::{Config, Pio, program::pio_file};
use defmt::{debug, trace, unimplemented};

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
    config.use_program(&program, &[scl]);
    config.set_in_pins(&[sda]);

    pio.sm0.set_config(&config);
    pio.sm0.set_pin_dirs(Direction::In, &[sda, scl]);

    debug!("Running PIO I2C slave program");
    pio.sm0.set_enable(true); // Start the state machine
    
    pio.irq0.wait().await; // Wait for the IRQ to be triggered
    debug!("I2C: start recieved");

    

    unimplemented!("Handle I2C slave communication here");
}
