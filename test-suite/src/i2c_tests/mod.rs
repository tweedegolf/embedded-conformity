use defmt::{Format, error, trace};

#[cfg(feature = "fp")]
use {
    crate::fp::{FPPeripherals, FPTest},
    embassy_rp::{i2c, pio},
    tester::{I2cSlaveTestError, I2cSlaveTester},
};

use crate::{
    TestError,
    dut::{DutPeripherals, DutTest},
    list_of_tests::TestSelector,
};

use embedded_hal::digital::OutputPin;
use embedded_hal::i2c::I2c;

pub const I2C_DEFAULT_ADDRESS: u8 = 0x55;

pub mod i2c_slave;
#[cfg(feature = "fp")]
pub mod tester;

#[derive(Format)]
pub enum I2cError<T> {
    InternalError(T),
    TestFailure(&'static str),
}

#[cfg(feature = "fp")]
impl From<I2cSlaveTestError> for I2cError<I2cSlaveTestError> {
    fn from(value: I2cSlaveTestError) -> Self {
        match value {
            I2cSlaveTestError::ExpectationFailure(msg) => I2cError::TestFailure(msg),
            err @ I2cSlaveTestError::InternalError(_) => I2cError::InternalError(err),
        }
    }
}

impl<T: Format + embedded_hal::i2c::ErrorType> From<T> for I2cError<T> {
    fn from(value: T) -> Self {
        I2cError::InternalError(value)
    }
}

pub mod simple_read {
    use super::*;

    const PAYLOAD: &[u8; 1] = &[13];

    /// The Device Under Test Test
    pub struct Dut;

    impl<P: OutputPin, T: I2c> DutTest<T, P> for Dut
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
                error!("i2c: payload mismatched what was read, got: {}, expected: {}", &buf, PAYLOAD);
                return Err(TestError::RunError);
            }

            Ok(())
        }

        fn teardown(&mut self, _: &mut DutPeripherals<T, P>) -> Result<(), TestError> {
            Ok(())
        }
    }

    /// The Fake Peripheral/Tester part of the test
    #[cfg(feature = "fp")]
    pub struct FP;

    #[cfg(feature = "fp")]
    impl<I: i2c::Instance, P: pio::Instance> FPTest<I, P> for FP {
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
}

pub mod simple_write {
    use super::*;

    const PAYLOAD: &[u8; 1] = &[13];

    /// The Device Under Test Test
    pub struct Dut;

    impl<P: OutputPin, T: I2c> DutTest<T, P> for Dut
    where
        T::Error: defmt::Format,
    {
        const S: TestSelector = TestSelector::I2C_SimpleWrite;

        fn setup(&mut self, _: &mut DutPeripherals<T, P>) -> Result<(), TestError> {
            Ok(())
        }

        fn run(&mut self, session: &mut DutPeripherals<T, P>) -> Result<(), TestError> {
            trace!("Starting i2c write");
            for _ in 0..3 {
                session
                    .i2c
                    .write(I2C_DEFAULT_ADDRESS, PAYLOAD)
                    .map_err(|e| {
                        error!("{}", e);
                        TestError::RunError
                    })?;
            }
            trace!("Finished i2c write");

            Ok(())
        }

        fn teardown(&mut self, _: &mut DutPeripherals<T, P>) -> Result<(), TestError> {
            Ok(())
        }
    }

    /// The Fake Peripheral/Tester part of the test
    #[cfg(feature = "fp")]
    pub struct FP;

    #[cfg(feature = "fp")]
    impl<I: i2c::Instance, P: pio::Instance> FPTest<I, P> for FP {
        const S: TestSelector = TestSelector::I2C_SimpleWrite;

        async fn setup(&mut self, _: &mut FPPeripherals<'_, I, P>) -> Result<(), TestError> {
            Ok(())
        }

        async fn run(&mut self, peripherals: &mut FPPeripherals<'_, I, P>) -> Result<(), TestError> {
            I2cSlaveTester::new(&mut peripherals.i2c)
                .expect_write(PAYLOAD)
                .expect_write(PAYLOAD)
                .expect_write(PAYLOAD)
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
}
