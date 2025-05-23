use crate::{DutTest, FPTest};
use defmt::{Format, info};
use embedded_hal::i2c::I2c;

#[cfg(feature = "fp")]
use embassy_rp::{
    i2c::Instance,
    i2c_slave::{self, I2cSlave},
};

pub const I2C_DEFAULT_ADDRESS: u8 = 0x55;

#[cfg(feature = "fp")]
pub mod tester;

#[derive(Format)]
pub enum I2cError {
    InternalError,
    TestFailure,
}

pub mod simple_read {
    use defmt::{Debug2Format, error, unwrap};

    #[cfg(feature = "fp")]
    use super::tester::I2cSlaveTester;

    use super::*;

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
        type E = I2cError;

        fn setup(&mut self) -> Result<(), I2cError> {
            Ok(())
        }

        fn run(&mut self) -> Result<(), I2cError> {
            let mut buf = [0; PAYLOAD.len()];
            self.0.read(I2C_DEFAULT_ADDRESS, &mut buf).map_err(|e| {
                error!("Error encountered: {}", &e);
                I2cError::InternalError
            })?;

            if &buf != PAYLOAD {
                return Err(I2cError::TestFailure);
            }

            Ok(())
        }

        fn teardown(&mut self) -> Result<(), I2cError> {
            Ok(())
        }
    }

    /// The Fake Peripheral/Tester part of the test
    #[cfg(feature = "fp")]
    pub struct FP<'a, 'b, I: Instance>(pub &'b mut I2cSlave<'a, I>);

    #[cfg(feature = "fp")]
    impl<I: Instance> FPTest for FP<'_, '_, I> {
        type E = I2cError;

        async fn setup(&mut self) -> Result<(), Self::E> {
            Ok(())
        }

        async fn run(&mut self) -> Result<(), Self::E> {
            unwrap!(I2cSlaveTester::new(self.0).expect_read(PAYLOAD).run().await);
            Ok(())
        }

        async fn teardown(&mut self) -> Result<(), Self::E> {
            self.0.reset();
            Ok(())
        }
    }
}

pub mod simple_write {
    use defmt::{error, unwrap};

    #[cfg(feature = "fp")]
    use super::tester::I2cSlaveTester;

    use super::*;

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
        type E = I2cError;

        fn setup(&mut self) -> Result<(), I2cError> {
            Ok(())
        }

        fn run(&mut self) -> Result<(), I2cError> {
            self.0.write(I2C_DEFAULT_ADDRESS, PAYLOAD).map_err(|e| {
                error!("Error encountered: {}", &e);
                I2cError::InternalError
            })?;

            Ok(())
        }

        fn teardown(&mut self) -> Result<(), I2cError> {
            Ok(())
        }
    }

    /// The Fake Peripheral/Tester part of the test
    #[cfg(feature = "fp")]
    pub struct FP<'a, 'b, I: Instance>(pub &'b mut I2cSlave<'a, I>);

    #[cfg(feature = "fp")]
    impl<I: Instance> FPTest for FP<'_, '_, I> {
        type E = I2cError;

        async fn setup(&mut self) -> Result<(), Self::E> {
            Ok(())
        }

        async fn run(&mut self) -> Result<(), Self::E> {
            unwrap!(
                I2cSlaveTester::new(self.0)
                    .expect_write(PAYLOAD)
                    .run()
                    .await
            );
            Ok(())
        }

        async fn teardown(&mut self) -> Result<(), Self::E> {
            self.0.reset();
            Ok(())
        }
    }
}
