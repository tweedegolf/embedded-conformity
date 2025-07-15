#![allow(non_camel_case_types)]
use embedded_hal::{digital::OutputPin, i2c::I2c};

use crate::{
    dut::{DutPeripherals, DutTest},
    i2c_tests::I2C_DEFAULT_ADDRESS,
    list_of_tests::TestSelector,
};
use defmt::{assert, assert_eq, debug, error, expect, info, intern, panic, trace, unwrap};

#[cfg(feature = "fp")]
use {
    crate::fp::{FPPeripherals, FPTest, PioPeripheral},
    crate::i2c_tests::tester::I2cSlaveTester,
    embassy_rp::{
        gpio::Pull,
        i2c,
        pio::{self, Config, Direction, ShiftConfig, ShiftDirection, program::pio_file},
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

    fn setup(&mut self, _: &mut DutPeripherals<T, P>) -> Result<(), ()> {
        Ok(())
    }

    fn run(&mut self, session: &mut DutPeripherals<T, P>) -> Result<(), ()> {
        trace!("Starting i2c write");
        session
            .i2c
            .write(I2C_DEFAULT_ADDRESS, &PAYLOAD)
            .map_err(|e| {
                error!("{}", e);
            })?;
        trace!("Finished i2c write");

        Ok(())
    }

    fn teardown(&mut self, _: &mut DutPeripherals<T, P>) -> Result<(), ()> {
        Ok(())
    }
}

#[cfg(feature = "fp")]
impl<I: i2c::Instance, P: pio::Instance> FPTest<I, P> for I2C_MultiWrite {
    const S: TestSelector = TestSelector::I2C_MultiWrite;

    async fn setup(&mut self, _: &mut FPPeripherals<'_, I, P>) -> Result<(), ()> {
        Ok(())
    }

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

        simple_init_pio(&mut peripherals.pio);

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

        loop {
            let rx = pio.sm0.rx().wait_pull().await;
            debug!("rx: {}", rx.to_be_bytes());
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
