use embedded_hal::digital::OutputPin;

use crate::{
    dut::{DutPeripherals, DutTest},
    list_of_tests::TestSelector,
};

#[cfg(feature = "fp")]
use {
    crate::fp::{FPPeripherals, FPTest},
    embassy_rp::{i2c, pio},
};

/// A simple sanity test that sets a pin high for the tester to check if it goes high
pub mod pin_test {
    use defmt::{debug, error};
    use embedded_hal::i2c::I2c;

    use super::*;

    /// The Device Under Test Test
    pub struct PinTest;

    impl<I2C: I2c, P: OutputPin> DutTest<I2C, P> for PinTest
    where
        <P as embedded_hal::digital::ErrorType>::Error: defmt::Format,
    {
        const S: TestSelector = TestSelector::Sanity_Pin;

        fn setup(&mut self, session: &mut DutPeripherals<I2C, P>) -> Result<(), ()> {
            session.pin.set_low().map_err(|e| {
                error!("{}", e);
            })
        }

        fn run(&mut self, session: &mut DutPeripherals<I2C, P>) -> Result<(), ()> {
            debug!("Set high");
            session.pin.set_high().map_err(|e| {
                error!("{}", e);
            })?;

            Ok(())
        }

        fn teardown(&mut self, session: &mut DutPeripherals<I2C, P>) -> Result<(), ()> {
            debug!("Set low");
            session.pin.set_low().map_err(|e| {
                error!("{}", e);
            })
        }
    }

    #[cfg(feature = "fp")]
    impl<I: i2c::Instance, P: pio::Instance> FPTest<I, P> for PinTest {
        const S: TestSelector = TestSelector::Sanity_Pin;

        async fn setup(&mut self, _: &mut FPPeripherals<'_, I, P>) -> Result<(), ()> {
            Ok(())
        }

        async fn run(&mut self, peripherals: &mut FPPeripherals<'_, I, P>) -> Result<(), ()> {
            peripherals.pin.wait_for_any_edge().await;
            Ok(())
        }

        async fn teardown(&mut self, _: &mut FPPeripherals<'_, I, P>) -> Result<(), ()> {
            Ok(())
        }
    }
}
