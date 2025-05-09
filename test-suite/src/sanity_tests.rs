use crate::{DutTest, FPTest};
use embedded_hal::digital::{InputPin, OutputPin};

/// A simple sanity test that sets a pin high for the tester to check if it goes high
pub mod pin_test {
    use super::*;

    /// The Device Under Test Test
    pub struct Dut<'a, T: OutputPin>(pub &'a mut T);

    impl<T: OutputPin> DutTest for Dut<'_, T> {
        type E = T::Error;

        fn setup(&mut self) -> Result<(), Self::E> {
            self.0.set_low()
        }

        fn run(&mut self) -> Result<(), Self::E> {
            self.0.set_high()
        }

        fn teardown(&mut self) -> Result<(), Self::E> {
            self.0.set_low()
        }
    }

    /// The Fake Peripheral/Tester part of the test
    #[cfg(feature = "fp")]
    pub struct FP<'a, T: InputPin>(pub &'a mut T);

    #[cfg(feature = "fp")]
    impl<T: InputPin> FPTest for FP<'_, T> {
        type E = T::Error;

        async fn setup(&mut self) -> Result<(), Self::E> {
            Ok(())
        }

        async fn run(&mut self) -> Result<(), Self::E> {
            while !self.0.is_high()? {}
            Ok(())
        }

        async fn teardown(&mut self) -> Result<(), Self::E> {
            Ok(())
        }
    }
}
