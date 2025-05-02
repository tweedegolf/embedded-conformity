use embedded_hal::digital::OutputPin;

pub fn test_one<T: OutputPin>(output: &mut T) -> Result<(), T::Error> {
    output.set_high()
}

