use crate::{DutTest, FPTest};
use embedded_hal::i2c::I2c;

#[cfg(feature = "fp")]
use embassy_rp::{
    i2c::Instance,
    i2c_slave::{self, I2cSlave},
};

pub const I2C_DEFAULT_ADDRESS: u8 = 0x55;

pub mod i2c_test_simple {

    use defmt::{debug, info, trace};

    use super::*;

    /// The Device Under Test Test
    pub struct Dut<'a, T: I2c>(pub &'a mut T);

    impl<T: I2c> DutTest for Dut<'_, T> {
        type E = T::Error;

        fn setup(&mut self) -> Result<(), Self::E> {
            Ok(())
        }

        fn run(&mut self) -> Result<(), Self::E> {
            let mut buf = [0; 1];
            info!("going to read");
            self.0.read(I2C_DEFAULT_ADDRESS, &mut buf)?;
            assert_eq!(&buf[0], &42);
            info!("got read");
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
        type E = i2c_slave::Error;

        async fn setup(&mut self) -> Result<(), Self::E> {
            Ok(())
        }

        async fn run(&mut self) -> Result<(), Self::E> {
            loop {
                match self.0.listen(&mut [0; 128]).await? {
                    i2c_slave::Command::GeneralCall(_) => trace!("general call"),
                    i2c_slave::Command::Read => { 
                        let status = self.0.respond_to_read(&[42]).await?;
                        match status {
                            i2c_slave::ReadStatus::Done => break,
                            i2c_slave::ReadStatus::NeedMoreBytes => {},
                            i2c_slave::ReadStatus::LeftoverBytes(_) => {},
                        }
                    },
                    i2c_slave::Command::WriteRead(_) => trace!("write read"),
                    i2c_slave::Command::Write(_) => trace!("write"),
                }
            }
            Ok(())
        }

        async fn teardown(&mut self) -> Result<(), Self::E> {
            self.0.reset();
            Ok(())
        }
    }
}
