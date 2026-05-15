use defmt::{error, unwrap};
use embedded_hal::{digital::OutputPin, i2c::I2c};
use rtt_target::UpChannel;

use crate::{
    Context, TestError,
    i2c_tests::{
        address_nak::I2C_AddressNAK,
        data_nak::I2C_DataNAK,
        multi_write::I2C_MultiWrite,
        simple_read::I2C_SimpleRead,
        simple_write::{I2C_SimpleWrite, I2C_SimpleWrite_PIO},
    },
    list_of_tests::TestSelector,
    protocol::{DUTToHost, HostToDUT, HostToDUTCommand, send_to_host},
    read_cobs,
    sanity_tests::pin_test::PinTest,
};

pub fn run_dut_tests<P: OutputPin, I2C: I2c>(mut ctx: Context, mut session: DutPeripherals<I2C, P>)
where
    <I2C as embedded_hal::i2c::ErrorType>::Error: defmt::Format,
    <P as embedded_hal::digital::ErrorType>::Error: defmt::Format,
{
    read_cobs(&mut ctx.channels.down, |data: HostToDUT| {
        send_to_host(DUTToHost::Ack(data.id), &mut ctx.channels.up);

        match data.command {
            HostToDUTCommand::Init => {}
            HostToDUTCommand::Run(TestSelector::Sanity_Pin) => {
                run_dut_test(PinTest, &mut ctx.channels.up, &mut session);
            }
            HostToDUTCommand::Run(TestSelector::I2C_SimpleRead) => {
                debug!("Running I2C Simple Read");
                run_dut_test(I2C_SimpleRead, &mut ctx.channels.up, &mut session);
            }
            HostToDUTCommand::Run(TestSelector::I2C_SimpleWrite) => {
                run_dut_test(I2C_SimpleWrite, &mut ctx.channels.up, &mut session);
            }
            HostToDUTCommand::Run(TestSelector::I2C_MultiWrite) => {
                run_dut_test(I2C_MultiWrite, &mut ctx.channels.up, &mut session);
            }
            HostToDUTCommand::Run(TestSelector::I2C_AddressNAK) => {
                run_dut_test(I2C_AddressNAK, &mut ctx.channels.up, &mut session);
            }
            HostToDUTCommand::Run(TestSelector::I2C_DataNAK) => {
                run_dut_test(I2C_DataNAK, &mut ctx.channels.up, &mut session);
            }
        }
    });
}

#[allow(
    clippy::useless_conversion,
    reason = ".into() is needed because with std feature the field is a String not a &str"
)]
fn run_dut_test<I2C: I2c, P: OutputPin, T: DutTest<I2C, P>>(
    mut test: T,
    up: &mut UpChannel,
    session: &mut DutPeripherals<I2C, P>,
) {
    let t = <T as DutTest<_, _>>::S;

    if let Err(e) = test.setup(session) {
        error!("Encountered error during setup of test {}: {:?}", t, &e);
        match e {
            TestError::Failure(msg) => send_to_host(DUTToHost::TestFailure(t, msg.into()), up),
            TestError::PartialSuccess(msg) => {
                send_to_host(DUTToHost::PartialSuccess(t, msg.into()), up)
            }
        }
        return;
    }

    if let Err(e) = test.run(session) {
        error!("Encountered error during run of test {}: {:?}", t, &e);
        match e {
            TestError::Failure(msg) => send_to_host(DUTToHost::TestFailure(t, msg.into()), up),
            TestError::PartialSuccess(msg) => {
                send_to_host(DUTToHost::PartialSuccess(t, msg.into()), up)
            }
        }
    }

    send_to_host(DUTToHost::Success(t), up);

    // we crash as we can not guarantee to correctness of the system
    unwrap!(test.teardown(session))
}

/// The Peripherals struct holds the I2C and pin used in the tests
pub struct DutPeripherals<I2C: I2c, P: OutputPin> {
    pub i2c: I2C,
    pub pin: P,
}

/// The DutTest trait defines the interface for a Device Under Test test
pub trait DutTest<I2C: I2c, P: OutputPin> {
    const S: TestSelector;

    fn setup(&mut self, _: &mut DutPeripherals<I2C, P>) -> Result<(), TestError> {
        Ok(())
    }

    fn run(&mut self, session: &mut DutPeripherals<I2C, P>) -> Result<(), TestError>;

    fn teardown(&mut self, _: &mut DutPeripherals<I2C, P>) -> Result<(), TestError> {
        Ok(())
    }
}
