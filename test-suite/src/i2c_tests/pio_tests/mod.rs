#![cfg(feature = "fp")]
#![allow(non_camel_case_types)]

use embassy_rp::{
    gpio::Pull,
    pio::{
        self,
        program::{Program, pio_file},
    },
};

use defmt::unwrap;

use crate::fp::PioPeripheral;
use embassy_rp::pio::Config;
use embassy_rp::pio::Direction;
use embassy_rp::pio::ShiftConfig;
use embassy_rp::pio::ShiftDirection;

fn init_pio<'a, const SIZE: usize, P: pio::Instance>(
    peripheral: &mut PioPeripheral<'a, P>,
    program: &Program<SIZE>,
    mut cfg: pio::Config<'a, P>,
) {
    let sda = &mut peripheral.sda;
    let scl = &mut peripheral.scl;
    let pio = &mut peripheral.pio;

    let program = pio.common.load_program(program);

    sda.set_pull(Pull::Up);
    scl.set_pull(Pull::Up);

    cfg.set_in_pins(&[sda, scl]);
    cfg.set_out_pins(&[sda]);
    cfg.set_set_pins(&[sda]);
    cfg.set_jmp_pin(sda);
    cfg.use_program(&program, &[sda]);

    pio.sm0.set_config(&cfg);

    pio.sm0.set_pin_dirs(Direction::In, &[sda, scl]);

    unwrap!(peripheral.programs.push(program).ok());
}

pub mod address_nak {
    use super::*;

    pub fn init_pio_address_nak<P: pio::Instance>(peripheral: &mut PioPeripheral<'_, P>) {
        let program = pio_file!(
            "src/i2c_tests/pio_tests/i2c_address_nak.pio",
            select_program("address_nak")
        );

        let mut config = Config::<P>::default();

        // Controls the RX FIFO
        config.shift_in = ShiftConfig {
            threshold: 8,
            direction: ShiftDirection::Left,
            auto_fill: true,
        };

        // Controls the TX FIFO
        config.shift_out = ShiftConfig {
            threshold: 8,
            direction: ShiftDirection::Left,
            auto_fill: true,
        };

        init_pio(peripheral, &program.program, config);
    }
}

pub mod simple_read_write {
    use super::*;

    // TODO: Possibly take parameters of TX/RX Thresholds
    pub fn simple_init_pio<P: pio::Instance>(
        peripheral: &mut PioPeripheral<'_, P>,
        tx_threshold: u8,
    ) {
        let program = pio_file!(
            "src/i2c_tests/pio_tests/i2c_simple.pio",
            select_program("i2c_slave")
        );

        let mut config = Config::<P>::default();

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

        init_pio(peripheral, &program.program, config);
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
