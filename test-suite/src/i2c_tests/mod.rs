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

pub mod pio_tests;
pub mod simple_read;
pub mod simple_write;

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
