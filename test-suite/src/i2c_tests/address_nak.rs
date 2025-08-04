#![allow(non_camel_case_types)]
use embedded_hal::{
    digital::OutputPin,
    i2c::{Error, I2c, NoAcknowledgeSource},
};

use crate::{
    dut::{DutPeripherals, DutTest},
    i2c_tests::I2C_DEFAULT_ADDRESS,
    list_of_tests::TestSelector,
};
use defmt::{assert, assert_eq, debug, error, expect, info, intern, panic, trace, unwrap, warn};

#[cfg(feature = "fp")]
use {
    crate::fp::{FPPeripherals, FPTest, PioPeripheral},
    crate::i2c_tests::pio_tests::simple_read_write::{simple_init_pio, simple_reset_pio},
    crate::i2c_tests::tester::I2cSlaveTester,
    embassy_rp::{
        gpio::Pull,
        i2c,
        pio::{self, Config, Direction, ShiftConfig, ShiftDirection, program::pio_file},
    },
};

/// The Device Under Test Test
pub struct I2C_AdressNAK;

impl<P: OutputPin, T: I2c> DutTest<T, P> for I2C_AdressNAK
where
    T::Error: defmt::Format,
{
    const S: TestSelector = TestSelector::I2C_AdressNAK;

    fn setup(&mut self, _: &mut DutPeripherals<T, P>) -> Result<(), ()> {
        Ok(())
    }

    fn run(&mut self, session: &mut DutPeripherals<T, P>) -> Result<(), ()> {
        info!("running test");
        match session
            .i2c
            .read(I2C_DEFAULT_ADDRESS, &mut [0; 1])
            .unwrap_err()
            .kind()
        {
            embedded_hal::i2c::ErrorKind::NoAcknowledge(NoAcknowledgeSource::Address) => {} // Expected
            embedded_hal::i2c::ErrorKind::NoAcknowledge(NoAcknowledgeSource::Unknown) => {
                warn!("I2C Unknown NACK Source for Address NACK");
            }
            _ => return Err(()),
        }

        Ok(())
    }

    fn teardown(&mut self, _: &mut DutPeripherals<T, P>) -> Result<(), ()> {
        Ok(())
    }
}

#[cfg(feature = "fp")]
impl<I: i2c::Instance, P: pio::Instance> FPTest<I, P> for I2C_AdressNAK {
    const S: TestSelector = TestSelector::I2C_AdressNAK;

    async fn setup(
        &mut self,
        peripherals: &mut crate::fp::FPPeripherals<'_, I, P>,
    ) -> Result<(), ()> {
        use crate::i2c_tests::pio_tests::address_nak::init_pio_address_nak;

        init_pio_address_nak(&mut peripherals.pio);

        Ok(())
    }

    async fn run(
        &mut self,
        peripherals: &mut crate::fp::FPPeripherals<'_, I, P>,
    ) -> Result<(), ()> {
        let pio = &mut peripherals.pio.pio;

        pio.sm0.set_enable(true); // Start the state machine
        pio.irq0.wait().await;
        let data = pio.sm0.rx().pull().to_be_bytes()[3];

        let address = data >> 1;
        let mode = data & 1 == 1; // true is read, false is write

        assert!(mode); // True == read
        assert_eq!(address, I2C_DEFAULT_ADDRESS);

        Ok(())
    }

    async fn teardown(
        &mut self,
        peripherals: &mut crate::fp::FPPeripherals<'_, I, P>,
    ) -> Result<(), ()> {
        simple_reset_pio(&mut peripherals.pio);
        Ok(())
    }
}
