use crate::{DutTest, FPTest};
use defmt::Format;
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
pub enum I2cError<E> {
    InternalError(E),
    TestFailure(&'static str),
}

pub mod i2c_test_simple {
    use defmt::{debug, info, trace, warn, unwrap};

    #[cfg(feature = "fp")]
    use super::tester::I2cSlaveTester;

    use super::*;

    /// The Device Under Test Test
    pub struct Dut<'a, T: I2c>(pub &'a mut T);

    impl<T: I2c> DutTest for Dut<'_, T>
    where
        <T as embedded_hal::i2c::ErrorType>::Error: Format,
    {
        type E = I2cError<T::Error>;

        fn setup(&mut self) -> Result<(), I2cError<T::Error>> {
            Ok(())
        }

        fn run(&mut self) -> Result<(), I2cError<T::Error>> {
            let mut buf = [0; 1];
            info!("going to read");
            self.0
                .read(I2C_DEFAULT_ADDRESS, &mut buf)
                .map_err(I2cError::InternalError)?;
            assert_eq!(&buf[0], &42);
            info!("got read");
            Ok(())
        }

        fn teardown(&mut self) -> Result<(), I2cError<T::Error>> {
            Ok(())
        }
    }

    /// The Fake Peripheral/Tester part of the test
    #[cfg(feature = "fp")]
    pub struct FP<'a, 'b, I: Instance>(pub &'b mut I2cSlave<'a, I>);

    #[cfg(feature = "fp")]
    impl<I: Instance> FPTest for FP<'_, '_, I> {
        type E = i2c_slave::Error;

        async fn setup(&mut self) -> Result<(), Self::E> {
            Ok(())
        }

        async fn run(&mut self) -> Result<(), Self::E> {
            unwrap!(I2cSlaveTester::new(self.0).expect_read(&[42]).run().await);
            Ok(())
        }

        async fn teardown(&mut self) -> Result<(), Self::E> {
            self.0.reset();
            Ok(())
        }
    }
}
