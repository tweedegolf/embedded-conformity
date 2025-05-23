use crate::{DutTest, FPTest};
use defmt::Format;
use embedded_hal::i2c::I2c;

#[cfg(feature = "fp")]
use embassy_rp::{i2c::Instance, i2c_slave::I2cSlave};
#[cfg(feature = "fp")]
use tester::I2cSlaveTestError;

pub const I2C_DEFAULT_ADDRESS: u8 = 0x55;

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

    use defmt::trace;
    #[cfg(feature = "fp")]
    use tester::{I2cSlaveTestError, I2cSlaveTester};

    const PAYLOAD: &[u8; 1] = &[13];

    /// The Device Under Test Test
    pub struct Dut<'a, T>(pub &'a mut T)
    where
        T: I2c,
        T::Error: Format;

    impl<T: I2c> DutTest for Dut<'_, T>
    where
        T::Error: defmt::Format,
    {
        type E = I2cError<T::Error>;

        fn setup(&mut self) -> Result<(), Self::E> {
            Ok(())
        }

        fn run(&mut self) -> Result<(), Self::E> {
            let mut buf = [0; PAYLOAD.len()];

            trace!("reading i2c");
            self.0
                .read(I2C_DEFAULT_ADDRESS, &mut buf)
                .map_err(I2cError::InternalError)?;
            trace!("done reading i2c");

            if &buf != PAYLOAD {
                return Err(I2cError::TestFailure("payload mismatched what was read"));
            }

            Ok(())
        }

        fn teardown(&mut self) -> Result<(), Self::E> {
            Ok(())
        }
    }

    /// The Fake Peripheral/Tester part of the test
    #[cfg(feature = "fp")]
    pub struct FP<'a, 'b, I: Instance>(pub &'b mut I2cSlave<'a, I>);

    #[cfg(feature = "fp")]
    impl<I: Instance> FPTest for FP<'_, '_, I> {
        type E = I2cError<I2cSlaveTestError>;

        async fn setup(&mut self) -> Result<(), Self::E> {
            Ok(())
        }

        async fn run(&mut self) -> Result<(), Self::E> {
            I2cSlaveTester::new(self.0)
                .expect_read(PAYLOAD)
                .run()
                .await?;
            Ok(())
        }

        async fn teardown(&mut self) -> Result<(), Self::E> {
            self.0.reset();
            Ok(())
        }
    }
}

pub mod simple_write {
    use super::*;

    use defmt::trace;
    #[cfg(feature = "fp")]
    use tester::I2cSlaveTester;

    const PAYLOAD: &[u8; 1] = &[13];

    /// The Device Under Test Test
    pub struct Dut<'a, T>(pub &'a mut T)
    where
        T: I2c,
        T::Error: Format;

    impl<T: I2c> DutTest for Dut<'_, T>
    where
        T::Error: defmt::Format,
    {
        type E = I2cError<T::Error>;

        fn setup(&mut self) -> Result<(), Self::E> {
            Ok(())
        }

        fn run(&mut self) -> Result<(), Self::E> {
            trace!("Starting i2c write");
            self.0
                .write(I2C_DEFAULT_ADDRESS, PAYLOAD)
                .map_err(I2cError::InternalError)?;
            trace!("Finished i2c write");

            Ok(())
        }

        fn teardown(&mut self) -> Result<(), Self::E> {
            Ok(())
        }
    }

    /// The Fake Peripheral/Tester part of the test
    #[cfg(feature = "fp")]
    pub struct FP<'a, 'b, I: Instance>(pub &'b mut I2cSlave<'a, I>);

    #[cfg(feature = "fp")]
    impl<I: Instance> FPTest for FP<'_, '_, I> {
        type E = I2cError<I2cSlaveTestError>;

        async fn setup(&mut self) -> Result<(), Self::E> {
            Ok(())
        }

        async fn run(&mut self) -> Result<(), Self::E> {
            I2cSlaveTester::new(self.0)
                .expect_write(PAYLOAD)
                .run()
                .await?;
            Ok(())
        }

        async fn teardown(&mut self) -> Result<(), Self::E> {
            self.0.reset();
            Ok(())
        }
    }
}

