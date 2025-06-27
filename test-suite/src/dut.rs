use core::hint::black_box;

use defmt::{debug, error, unwrap};
use embedded_hal::{digital::OutputPin, i2c::I2c};
use rtt_target::UpChannel;

use crate::{
    Context, i2c_tests,
    list_of_tests::TestSelector,
    protocol::{DUTToHost, HostToDUT, HostToDUTCommand, send_to_host},
    read_cobs, sanity_tests,
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
            HostToDUTCommand::Run(t @ TestSelector::Sanity_Pin) => {
                debug!("running test {}", t);
                let test = sanity_tests::pin_test::Dut;
                run_dut_test(t, test, &mut ctx.channels.up, &mut session);
            }
            HostToDUTCommand::Run(t @ TestSelector::I2C_SimpleRead) => {
                debug!("running test {:?}", t);
                let test = i2c_tests::simple_read::SimpleRead;
                run_dut_test(t, test, &mut ctx.channels.up, &mut session);
            }
            HostToDUTCommand::Run(t @ TestSelector::I2C_SimpleWrite) => {
                debug!("running test {:?}", t);
                let test = i2c_tests::simple_write::Dut;
                run_dut_test(t, test, &mut ctx.channels.up, &mut session);
            }
        }
    });
}

fn run_dut_test<I2C: I2c, P: OutputPin>(
    t: TestSelector,
    mut test: impl DutTest<I2C, P>,
    up: &mut UpChannel,
    session: &mut DutPeripherals<I2C, P>,
) {
    if let Err(e) = test.setup(session) {
        error!("Encountered error during setup of test {}: {:?}", t, &e);
        send_to_host(DUTToHost::TestFailure(t), up);
        return;
    }

    if let Err(e) = test.run(session) {
        error!("Encountered error during run of test {}: {:?}", t, &e);
        send_to_host(DUTToHost::TestFailure(t), up);
        return;
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

    fn setup(&mut self, session: &mut DutPeripherals<I2C, P>) -> Result<(), ()>;
    fn run(&mut self, session: &mut DutPeripherals<I2C, P>) -> Result<(), ()>;
    fn teardown(&mut self, session: &mut DutPeripherals<I2C, P>) -> Result<(), ()>;
}
