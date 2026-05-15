#![allow(non_camel_case_types)]
use embedded_hal::{digital::OutputPin, i2c::I2c};

use crate::{
    TestError,
    dut::{DutPeripherals, DutTest},
    i2c_tests::I2C_DEFAULT_ADDRESS,
    list_of_tests::TestSelector,
};
use defmt::error;

#[cfg(feature = "fp")]
use {
    crate::fp::{FPPeripherals, FPTest},
    crate::i2c_tests::tester::I2cSlaveTester,
    embassy_rp::{
        i2c,
        pio::{self},
    },
};

const PAYLOAD: [u8; 4] = [56, 12, 42, 18];

pub struct I2C_MultiWrite;
pub struct I2C_MultiWrite_PIO;

impl<P: OutputPin, T: I2c> DutTest<T, P> for I2C_MultiWrite
where
    T::Error: defmt::Format,
{
    const S: TestSelector = TestSelector::I2C_MultiWrite;

    fn run(&mut self, session: &mut DutPeripherals<T, P>) -> Result<(), TestError> {
        session
            .i2c
            .write(I2C_DEFAULT_ADDRESS, &PAYLOAD)
            .map_err(|e| {
                error!("{}", e);
                TestError::Failure("i2c failed to write")
            })?;

        Ok(())
    }
}

#[cfg(feature = "fp")]
impl<I: i2c::Instance, P: pio::Instance> FPTest<I, P> for I2C_MultiWrite {
    const S: TestSelector = TestSelector::I2C_MultiWrite;

    async fn run(&mut self, peripherals: &mut FPPeripherals<'_, I, P>) -> Result<(), ()> {
        I2cSlaveTester::new(&mut peripherals.i2c)
            .expect_write(&PAYLOAD)
            .run()
            .await
            .map_err(|e| {
                error!("{}", e);
            })?;
        Ok(())
    }

    async fn teardown(&mut self, peripherals: &mut FPPeripherals<'_, I, P>) -> Result<(), ()> {
        peripherals.i2c.reset();
        Ok(())
    }
}

#[cfg(feature = "fp")]
impl<I: i2c::Instance, P: pio::Instance> FPTest<I, P> for I2C_MultiWrite_PIO {
    const S: TestSelector = TestSelector::I2C_MultiWrite;

    async fn setup(
        &mut self,
        peripherals: &mut crate::fp::FPPeripherals<'_, I, P>,
    ) -> Result<(), ()> {
        use crate::i2c_tests::pio_tests::simple_read_write::simple_init_pio;

        simple_init_pio(&mut peripherals.pio, 32);

        let pio = &mut peripherals.pio.pio;
        pio.sm0.tx().push(0u32.to_be()); // The Reply, 0 -> None

        Ok(())
    }

    async fn run(
        &mut self,
        peripherals: &mut crate::fp::FPPeripherals<'_, I, P>,
    ) -> Result<(), ()> {
        let pio = &mut peripherals.pio.pio;

        pio.irq_flags.clear_all(0xF);
        pio.sm0.set_enable(true); // Start the state machine

        let data = pio.sm0.rx().wait_pull().await.to_be_bytes()[3];
        let address = data >> 1;
        let mode = data & 1 == 1; // true is read, false is write

        assert!(!mode); // True == read
        assert_eq!(address, I2C_DEFAULT_ADDRESS);

        for el in PAYLOAD {
            let rx = pio.sm0.rx().wait_pull().await.to_be_bytes()[3];
            assert_eq!(rx, el);
        }

        Ok(())
    }

    async fn teardown(
        &mut self,
        peripherals: &mut crate::fp::FPPeripherals<'_, I, P>,
    ) -> Result<(), ()> {
        use crate::i2c_tests::pio_tests::simple_read_write::simple_reset_pio;

        simple_reset_pio(&mut peripherals.pio);
        Ok(())
    }
}
