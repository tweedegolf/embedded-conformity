use embedded_hal::digital::OutputPin;

use crate::{
    dut::{DutPeripherals, DutTest},
    list_of_tests::TestSelector,
};

#[cfg(feature = "fp")]
use {
    crate::fp::{FPPeripherals, FPTest},
    embassy_rp::i2c::Instance,
};

/// A simple sanity test that sets a pin high for the tester to check if it goes high
pub mod pin_test {
    use defmt::error;
    use embedded_hal::i2c::I2c;

    use crate::TestError;

    use super::*;

    /// The Device Under Test Test
    pub struct Dut;

    impl<I2C: I2c, P: OutputPin> DutTest<I2C, P> for Dut
    where
        <P as embedded_hal::digital::ErrorType>::Error: defmt::Format,
    {
        const S: TestSelector = TestSelector::Sanity_Pin;

        fn setup(&mut self, session: &mut DutPeripherals<I2C, P>) -> Result<(), TestError> {
            session.pin.set_low().map_err(|e| {
                error!("{}", e);
                TestError::SetupError
            })
        }

        fn run(&mut self, session: &mut DutPeripherals<I2C, P>) -> Result<(), TestError> {
            session.pin.set_high().map_err(|e| {
                error!("{}", e);
                TestError::RunError
            })
        }

        fn teardown(&mut self, session: &mut DutPeripherals<I2C, P>) -> Result<(), TestError> {
            session.pin.set_low().map_err(|e| {
                error!("{}", e);
                TestError::TeardownError
            })
        }
    }

    /// The Fake Peripheral/Tester part of the test
    #[cfg(feature = "fp")]
    pub struct FP;

    #[cfg(feature = "fp")]
    impl<I: Instance> FPTest<I> for FP {
        const S: TestSelector = TestSelector::Sanity_Pin;

        async fn setup(&mut self, _: &mut FPPeripherals<'_, I>) -> Result<(), TestError> {
            Ok(())
        }

        async fn run(&mut self, peripherals: &mut FPPeripherals<'_, I>) -> Result<(), TestError> {
            while peripherals.pin.is_low() {}
            Ok(())
        }

        async fn teardown(&mut self, _: &mut FPPeripherals<'_, I>) -> Result<(), TestError> {
            Ok(())
        }
    }
}
