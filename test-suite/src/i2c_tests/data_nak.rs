#![allow(non_camel_case_types)]
use embedded_hal::{
    digital::OutputPin,
    i2c::{Error, I2c, NoAcknowledgeSource},
};

use crate::{
    TestError,
    dut::{DutPeripherals, DutTest},
    i2c_tests::I2C_DEFAULT_ADDRESS,
    list_of_tests::TestSelector,
};

#[cfg(feature = "fp")]
use {
    crate::fp::FPTest,
    crate::i2c_tests::pio_tests::simple_read_write::simple_reset_pio,
    embassy_rp::{
        i2c,
        pio::{self},
    },
};

/// The Device Under Test Test
const PAYLOAD: u8 = 87;
pub struct I2C_DataNAK;

impl<P: OutputPin, T: I2c> DutTest<T, P> for I2C_DataNAK
where
    T::Error: defmt::Format,
{
    const S: TestSelector = TestSelector::I2C_DataNAK;

    fn run(&mut self, session: &mut DutPeripherals<T, P>) -> Result<(), TestError> {
        if let Err(e) = session.i2c.write(I2C_DEFAULT_ADDRESS, &[PAYLOAD]) {
            match e.kind() {
                embedded_hal::i2c::ErrorKind::NoAcknowledge(NoAcknowledgeSource::Data) => Ok(()),
                embedded_hal::i2c::ErrorKind::NoAcknowledge(NoAcknowledgeSource::Unknown) => Err(
                    TestError::PartialSuccess("I2C Unknown NACK Source for Address NACK"),
                ),
                _ => Err(TestError::Failure("wrong nack error")),
            }
        } else {
            Err(TestError::Failure("no nack error"))
        }
    }
}

#[cfg(feature = "fp")]
impl<I: i2c::Instance, P: pio::Instance> FPTest<I, P> for I2C_DataNAK {
    const S: TestSelector = TestSelector::I2C_DataNAK;

    async fn setup(
        &mut self,
        peripherals: &mut crate::fp::FPPeripherals<'_, I, P>,
    ) -> Result<(), ()> {
        use crate::i2c_tests::pio_tests::data_nak::init_pio_data_nak;

        init_pio_data_nak(&mut peripherals.pio);

        Ok(())
    }

    async fn run(
        &mut self,
        peripherals: &mut crate::fp::FPPeripherals<'_, I, P>,
    ) -> Result<(), ()> {
        let pio = &mut peripherals.pio.pio;

        pio.sm0.set_enable(true); // Start the state machine

        let data = pio.sm0.rx().wait_pull().await.to_be_bytes()[3];
        let address = data >> 1;
        let mode = data & 1 == 1; // true is read, false is write

        assert!(!mode); // True == read
        assert_eq!(address, I2C_DEFAULT_ADDRESS);

        let write = pio.sm0.rx().wait_pull().await;

        assert_eq!(PAYLOAD, write.to_be_bytes()[3]);

        pio.irq0.wait().await;

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
