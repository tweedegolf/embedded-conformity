#![cfg(feature = "fp")]
#![allow(non_camel_case_types)]

use embassy_rp::{
    gpio::Pull,
    pio::{self, program::pio_file},
};

use defmt::unwrap;

use crate::fp::PioPeripheral;
use embassy_rp::pio::Config;
use embassy_rp::pio::Direction;
use embassy_rp::pio::ShiftConfig;
use embassy_rp::pio::ShiftDirection;

pub mod simple_read_write {
    use super::*;

    // TODO: Possibly take parameters of TX/RX Thresholds
    pub fn simple_init_pio<P: pio::Instance>(peripheral: &mut PioPeripheral<'_, P>, tx_threshold: u8) {
        let sda = &mut peripheral.sda;
        let scl = &mut peripheral.scl;
        let pio = &mut peripheral.pio;

        let program = pio_file!(
            "src/i2c_tests/pio_tests/i2c_simple.pio",
            select_program("i2c_slave")
        );
        let program = pio.common.load_program(&program.program);

        // i2c requires pull-up
        sda.set_pull(Pull::Up);
        scl.set_pull(Pull::Up);

        let mut config = Config::<P>::default();
        config.set_in_pins(&[sda, scl]);
        config.set_out_pins(&[sda]);
        config.set_set_pins(&[sda]);
        config.set_jmp_pin(sda);
        config.use_program(&program, &[sda]);

        // Controls the RX FIFO
        config.shift_in = ShiftConfig {
            threshold: 8,
            direction: ShiftDirection::Left,
            auto_fill: true,
        };

        // Controls the TX FIFO
        config.shift_out = ShiftConfig {
            threshold: tx_threshold,
            direction: ShiftDirection::Left,
            auto_fill: true,
        };

        pio.sm0.set_config(&config);
        pio.sm0.set_pin_dirs(Direction::In, &[sda, scl]);

        unwrap!(peripheral.programs.push(program).ok());
    }

    pub fn simple_reset_pio<P: pio::Instance>(peripheral: &mut PioPeripheral<'_, P>) {
        let program = unwrap!(peripheral.programs.pop());

        let pio = &mut peripheral.pio;
        pio.sm0.set_enable(false);
        pio.irq_flags.clear_all(0xF);
        pio.sm0.clear_fifos();

        // Safety: The PIO is stopped
        unsafe { pio.common.free_instr(program.used_memory) };

        pio.sm0.restart();
    }
}
