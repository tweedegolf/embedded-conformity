#![cfg(feature = "fp")]

use defmt::{assert, assert_eq, debug, error, info, intern, panic, trace};
use embassy_futures::select::{Either4, select4};
use embassy_rp::gpio::Pull;
use embassy_rp::pio::{Config, Pio, program::pio_file};
use embassy_rp::pio::{Direction, InstanceMemory, ShiftConfig, ShiftDirection};
use embassy_rp::{i2c, pio};

use crate::TestError;
use crate::fp::FPTest;
use crate::list_of_tests::TestSelector;

pub struct I2C_SimpleRead_PIO;

impl<I: i2c::Instance, P: pio::Instance> FPTest<I, P> for I2C_SimpleRead_PIO {
    const S: TestSelector = TestSelector::I2C_SimpleRead;

    async fn setup(
        &mut self,
        peripherals: &mut crate::fp::FPPeripherals<'_, I, P>,
    ) -> Result<(), crate::TestError> {
        let pio = &mut peripherals.pio.pio;
        let sda = &mut peripherals.pio.sda;
        let scl = &mut peripherals.pio.scl;

        let program = pio_file!("src/i2c_tests/i2c_simple.pio", select_program("i2c_slave"));
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
        config.shift_in = ShiftConfig {
            threshold: 8,
            direction: ShiftDirection::Left,
            auto_fill: true,
        };
        config.shift_out = ShiftConfig {
            threshold: 8,
            direction: ShiftDirection::Left,
            auto_fill: true,
        };

        pio.sm0.set_config(&config);
        pio.sm0.set_pin_dirs(Direction::In, &[sda, scl]);

        pio.sm0.tx().push(13u32.to_be()); // The Reply

        Ok(())
    }

    async fn run(
        &mut self,
        peripherals: &mut crate::fp::FPPeripherals<'_, I, P>,
    ) -> Result<(), crate::TestError> {
        let pio = &mut peripherals.pio.pio;

        pio.sm0.set_enable(true); // Start the state machine
        let data = pio.sm0.rx().wait_pull().await.to_be_bytes()[3];

        let address = data >> 1;
        let mode = data & 1 == 1; // true is read, false is write

        assert!(mode); // True == read
        assert_eq!(address, 0x55);

        pio.irq3.wait().await;

        Ok(())
    }

    async fn teardown(
        &mut self,
        peripherals: &mut crate::fp::FPPeripherals<'_, I, P>,
    ) -> Result<(), crate::TestError> {
        peripherals.pio.pio.sm0.set_enable(false);
        peripherals.pio.pio.irq_flags.clear_all(0xF);
        peripherals.pio.pio.sm0.clear_fifos();
        peripherals.pio.pio.sm0.restart();

        Ok(())
    }
}

pub struct I2C_SimpleWrite_PIO;

impl<I: i2c::Instance, P: pio::Instance> FPTest<I, P> for I2C_SimpleWrite_PIO {
    const S: TestSelector = TestSelector::I2C_SimpleWrite;

    async fn setup(
        &mut self,
        peripherals: &mut crate::fp::FPPeripherals<'_, I, P>,
    ) -> Result<(), crate::TestError> {
        let pio = &mut peripherals.pio.pio;
        pio.sm0.tx().push(0u32.to_be()); // The Reply, 0 -> None

        Ok(())
    }

    async fn run(
        &mut self,
        peripherals: &mut crate::fp::FPPeripherals<'_, I, P>,
    ) -> Result<(), crate::TestError> {
        let pio = &mut peripherals.pio.pio;

        pio.sm0.set_enable(true); // Start the state machine

        let data = pio.sm0.rx().wait_pull().await.to_be_bytes()[3];
        let address = data >> 1;
        let mode = data & 1 == 1; // true is read, false is write

        assert!(!mode); // True == read
        assert_eq!(address, 0x55);

        let write = pio.sm0.rx().wait_pull().await;

        assert_eq!(13, write.to_be_bytes()[3]);

        pio.irq3.wait().await;

        debug!("finished i2c::write");

        Ok(())
    }

    async fn teardown(
        &mut self,
        peripherals: &mut crate::fp::FPPeripherals<'_, I, P>,
    ) -> Result<(), crate::TestError> {
        peripherals.pio.pio.sm0.set_enable(false);
        peripherals.pio.pio.sm0.clear_fifos();

        Ok(())
    }
}
