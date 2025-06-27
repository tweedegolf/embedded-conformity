#![allow(non_camel_case_types)]
use embedded_hal::{digital::OutputPin, i2c::I2c};

use crate::{
    TestError,
    dut::{DutPeripherals, DutTest},
    i2c_tests::I2C_DEFAULT_ADDRESS,
    list_of_tests::TestSelector,
};
use defmt::{assert, assert_eq, debug, error, expect, info, intern, panic, trace, unwrap};

#[cfg(feature = "fp")]
use {
    crate::fp::{FPPeripherals, FPTest, PioPeripheral},
    crate::i2c_tests::pio_tests::simple_read_write::{simple_init_pio, simple_reset_pio},
    crate::i2c_tests::tester::I2cSlaveTester,
    embassy_rp::{
        gpio::Pull,
        i2c,
        pio::{self, Config, Direction, ShiftConfig, ShiftDirection, program::pio_file},
    },
};

const PAYLOAD: &[u8; 1] = &[13];

/// The Device Under Test Test
pub struct SimpleRead;

impl<P: OutputPin, T: I2c> DutTest<T, P> for SimpleRead
where
    T::Error: defmt::Format,
{
    const S: TestSelector = TestSelector::I2C_SimpleRead;

    fn setup(&mut self, _: &mut DutPeripherals<T, P>) -> Result<(), TestError> {
        Ok(())
    }

    fn run(&mut self, session: &mut DutPeripherals<T, P>) -> Result<(), TestError> {
        let mut buf = [0; PAYLOAD.len()];

        session
            .i2c
            .read(I2C_DEFAULT_ADDRESS, &mut buf)
            .map_err(|e| {
                error!("{}", e);
                TestError::RunError
            })?;

        if &buf != PAYLOAD {
            error!(
                "i2c: payload mismatched what was read, got: {}, expected: {}",
                &buf, PAYLOAD
            );
            return Err(TestError::RunError);
        }

        Ok(())
    }

    fn teardown(&mut self, _: &mut DutPeripherals<T, P>) -> Result<(), TestError> {
        Ok(())
    }
}

#[cfg(feature = "fp")]
impl<I: i2c::Instance, P: pio::Instance> FPTest<I, P> for SimpleRead {
    const S: TestSelector = TestSelector::I2C_SimpleRead;

    async fn setup(&mut self, _: &mut FPPeripherals<'_, I, P>) -> Result<(), TestError> {
        Ok(())
    }

    async fn run(&mut self, peripherals: &mut FPPeripherals<'_, I, P>) -> Result<(), TestError> {
        I2cSlaveTester::new(&mut peripherals.i2c)
            .expect_read(PAYLOAD)
            .run()
            .await
            .map_err(|e| {
                error!("{}", e);
                TestError::RunError
            })?;
        Ok(())
    }

    async fn teardown(
        &mut self,
        peripherals: &mut FPPeripherals<'_, I, P>,
    ) -> Result<(), TestError> {
        peripherals.i2c.reset();
        Ok(())
    }
}

/// The PIO implementation of the I2C Simple Read test
pub struct I2C_SimpleRead_PIO;

#[cfg(feature = "fp")]
impl<I: i2c::Instance, P: pio::Instance> FPTest<I, P> for I2C_SimpleRead_PIO {
    const S: TestSelector = TestSelector::I2C_SimpleRead;

    async fn setup(
        &mut self,
        peripherals: &mut crate::fp::FPPeripherals<'_, I, P>,
    ) -> Result<(), crate::TestError> {
        simple_init_pio(&mut peripherals.pio);

        peripherals.pio.pio.sm0.tx().push(13u32.to_be()); // The Reply

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
        assert_eq!(address, I2C_DEFAULT_ADDRESS);

        pio.irq3.wait().await;

        Ok(())
    }

    async fn teardown(
        &mut self,
        peripherals: &mut crate::fp::FPPeripherals<'_, I, P>,
    ) -> Result<(), crate::TestError> {
        simple_reset_pio(&mut peripherals.pio);
        Ok(())
    }
}
