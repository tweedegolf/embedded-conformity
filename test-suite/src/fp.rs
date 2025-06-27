#![cfg(feature = "fp")]

use defmt::{debug, error, trace, unwrap};

pub use embassy_rp;

use embassy_rp::{
    gpio::Input,
    i2c,
    i2c_slave::I2cSlave,
    pio::{self, InstanceMemory, LoadedProgram, Pio},
};
use heapless::Vec;
use postcard::accumulator::{CobsAccumulator, FeedResult};
use rtt_target::UpChannel;

use crate::{
    Context, TestError,
    i2c_tests::{simple_read::I2C_SimpleRead_PIO, simple_write::I2C_SimpleWrite_PIO},
    list_of_tests::TestSelector,
    protocol::{FPToHost, HostToFP, HostToFPCommand, send_to_host},
    sanity_tests,
};

pub struct FPPeripherals<'a, I: i2c::Instance, P: pio::Instance> {
    pub i2c: I2cSlave<'a, I>,
    pub pin: Input<'a>,
    pub pio: PioPeripheral<'a, P>,
}

pub struct PioPeripheral<'a, P: pio::Instance> {
    pub pio: Pio<'a, P>,
    pub scl: pio::Pin<'a, P>,
    pub sda: pio::Pin<'a, P>,
    pub programs: Vec<LoadedProgram<'a, P>, 4>,
}

/// The FPTest trait defines the interface for the fake peripheral side of the tests
pub trait FPTest<I: i2c::Instance, P: pio::Instance> {
    /// Specifies which test this is,
    const S: TestSelector;

    async fn setup(&mut self, peripherals: &mut FPPeripherals<'_, I, P>) -> Result<(), TestError>;
    async fn run(&mut self, peripherals: &mut FPPeripherals<'_, I, P>) -> Result<(), TestError>;
    async fn teardown(
        &mut self,
        peripherals: &mut FPPeripherals<'_, I, P>,
    ) -> Result<(), TestError>;
}

async fn run_fp_test<I: i2c::Instance, P: pio::Instance>(
    t: TestSelector,
    mut test: impl FPTest<I, P>,
    up: &mut UpChannel,
    peripherals: &mut FPPeripherals<'_, I, P>,
) {
    if let Err(e) = test.setup(peripherals).await {
        error!("Encountered error during setup of test {}: {:?}", t, &e);
        send_to_host(FPToHost::TestFailure(t), up);
        return;
    }

    if let Err(e) = test.run(peripherals).await {
        error!("Encountered error during run of test {}: {:?}", t, &e);
        send_to_host(FPToHost::TestFailure(t), up);
        return;
    }

    send_to_host(FPToHost::Success(t), up);

    // we crash as we can not guarantee to correctness of the system
    unwrap!(test.teardown(peripherals).await)
}

pub async fn run_fp_tests<I: i2c::Instance, P: pio::Instance>(
    mut ctx: Context,
    mut peripherals: FPPeripherals<'_, I, P>,
) {
    let mut raw_buf = [0u8; 128];
    let mut cobs_buf: CobsAccumulator<256> = CobsAccumulator::new();

    loop {
        let ct = ctx.channels.down.read(&mut raw_buf);
        // Finished reading input
        if ct == 0 {
            continue;
        }

        let buf = &raw_buf[..ct];
        let mut window = buf;

        'cobs: while !window.is_empty() {
            window = match cobs_buf.feed::<HostToFP>(window) {
                FeedResult::Consumed => break 'cobs,
                FeedResult::OverFull(new_wind) => new_wind,
                FeedResult::DeserError(new_wind) => new_wind,
                FeedResult::Success { data, remaining } => {
                    send_to_host(FPToHost::Ack(data.id), &mut ctx.channels.up);
                    match data.command {
                        HostToFPCommand::Init => {}
                        HostToFPCommand::Run(t @ TestSelector::Sanity_Pin) => {
                            debug!("running test {}", t);
                            let test = sanity_tests::pin_test::FP;
                            run_fp_test(t, test, &mut ctx.channels.up, &mut peripherals).await;
                        }
                        HostToFPCommand::Run(t @ TestSelector::I2C_SimpleRead) => {
                            debug!("running test {}", t);
                            run_fp_test(
                                t,
                                I2C_SimpleRead_PIO,
                                &mut ctx.channels.up,
                                &mut peripherals,
                            )
                            .await;
                        }
                        HostToFPCommand::Run(t @ TestSelector::I2C_SimpleWrite) => {
                            debug!("running test {}", t);
                            run_fp_test(
                                t,
                                I2C_SimpleWrite_PIO,
                                &mut ctx.channels.up,
                                &mut peripherals,
                            )
                            .await;
                        }
                    }
                    remaining
                }
            };
        }
    }
}
