#![allow(non_camel_case_types)]
use embedded_hal::{digital::OutputPin, i2c::I2c};

use crate::{
    TestError,
    dut::{DutPeripherals, DutTest},
    i2c_tests::I2C_DEFAULT_ADDRESS,
    list_of_tests::TestSelector,
};
use defmt::error;

#[cfg(feature = "fp")]
use {
    crate::fp::{FPPeripherals, FPTest},
    crate::i2c_tests::pio_tests::simple_read_write::{simple_init_pio, simple_reset_pio},
    crate::i2c_tests::tester::I2cSlaveTester,
    embassy_rp::{
        i2c,
        pio::{self},
    },
};

const PAYLOAD: u8 = 13;

/// The Device Under Test Test
pub struct I2C_SimpleRead;
pub struct I2C_SimpleRead_PIO;

impl<P: OutputPin, T: I2c> DutTest<T, P> for I2C_SimpleRead
where
    T::Error: defmt::Format,
{
    const S: TestSelector = TestSelector::I2C_SimpleRead;

    fn run(&mut self, session: &mut DutPeripherals<T, P>) -> Result<(), TestError> {
        let mut buf = [0; 1];

        session.i2c.read(I2C_DEFAULT_ADDRESS, &mut buf).unwrap();

        if buf[0] != PAYLOAD {
            error!(
                "i2c: payload mismatched what was read, got: {}, expected: {}",
                &buf, PAYLOAD
            );

            Err(TestError::Failure("i2c payload mismatch"))
        } else {
            Ok(())
        }
    }
}

#[cfg(feature = "fp")]
impl<I: i2c::Instance, P: pio::Instance> FPTest<I, P> for I2C_SimpleRead {
    const S: TestSelector = TestSelector::I2C_SimpleRead;

    async fn run(&mut self, peripherals: &mut FPPeripherals<'_, I, P>) -> Result<(), ()> {
        I2cSlaveTester::new(&mut peripherals.i2c)
            .expect_read(&[PAYLOAD])
            .run()
            .await
            .map_err(|e| {
                error!("{}", e);
            })?;
        Ok(())
    }
}

// The PIO implementation of the I2C Simple Read test
#[cfg(feature = "fp")]
impl<I: i2c::Instance, P: pio::Instance> FPTest<I, P> for I2C_SimpleRead_PIO {
    const S: TestSelector = TestSelector::I2C_SimpleRead;

    async fn setup(
        &mut self,
        peripherals: &mut crate::fp::FPPeripherals<'_, I, P>,
    ) -> Result<(), ()> {
        simple_init_pio(&mut peripherals.pio, 8);

        let payload = (PAYLOAD as u32).to_be();
        peripherals.pio.pio.sm0.tx().push(payload); // The Reply

        Ok(())
    }

    async fn run(
        &mut self,
        peripherals: &mut crate::fp::FPPeripherals<'_, I, P>,
    ) -> Result<(), ()> {
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
    ) -> Result<(), ()> {
        simple_reset_pio(&mut peripherals.pio);
        Ok(())
    }
}
