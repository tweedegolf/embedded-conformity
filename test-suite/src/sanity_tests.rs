use embedded_hal::digital::OutputPin;

use crate::Test;

pub struct TestOne<'a, T: OutputPin>(pub &'a mut T);

impl<T: OutputPin> Test for TestOne<'_, T> {
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
