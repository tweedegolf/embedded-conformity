#![cfg(feature = "fp")]

use defmt::{debug, error, trace, unwrap};
pub use embassy_rp;
use embassy_rp::{gpio::Input, i2c, i2c_slave::I2cSlave, pio::{self, Pio}};
use postcard::accumulator::{CobsAccumulator, FeedResult};
use rtt_target::UpChannel;

use crate::{
    i2c_tests::{self, i2c_slave::test_pio_i2c_slave}, list_of_tests::TestSelector, protocol::{send_to_host, FPToHost, HostToFP, HostToFPCommand}, sanity_tests, Context, TestError
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
}

/// The FPTest trait defines the interface for the fake peripheral side of the tests
pub trait FPTest<I: i2c::Instance, P: pio::Instance> {
    const S: TestSelector;

    async fn setup(&mut self, peripherals: &mut FPPeripherals<'_, I, P>) -> Result<(), TestError>;
    async fn run(&mut self, peripherals: &mut FPPeripherals<'_, I, P>) -> Result<(), TestError>;
    async fn teardown(&mut self, peripherals: &mut FPPeripherals<'_, I, P>) -> Result<(), TestError>;
}

async fn run_fp_test<I: i2c::Instance, P: pio::Instance>(
    n: u32,
    mut test: impl FPTest<I, P>,
    up: &mut UpChannel,
    peripherals: &mut FPPeripherals<'_, I, P>,
) {
    if let Err(e) = test.setup(peripherals).await {
        error!("Encountered error during setup of test {}: {:?}", n, &e);
        send_to_host(FPToHost::TestFailure(n), up);
        return;
    }

    if let Err(e) = test.run(peripherals).await {
        error!("Encountered error during run of test {}: {:?}", n, &e);
        send_to_host(FPToHost::TestFailure(n), up);
        return;
    }

    send_to_host(FPToHost::Success(n), up);

    // we crash as we can not guarantee to correctness of the system
    unwrap!(test.teardown(peripherals).await)
}

pub async fn run_fp_tests<I: i2c::Instance, P: pio::Instance>(mut ctx: Context, mut peripherals: FPPeripherals<'_, I, P>) {
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
                        HostToFPCommand::Run(n @ 0) => {
                            debug!("running test {}", n);
                            let test = sanity_tests::pin_test::FP;
                            run_fp_test(n, test, &mut ctx.channels.up, &mut peripherals).await;
                        }
                        HostToFPCommand::Run(n @ 1) => {
                            debug!("running test {}", n);
                            test_pio_i2c_slave(
                                &mut peripherals.pio.pio,
                                &mut peripherals.pio.sda,
                                &mut peripherals.pio.scl,
                            ).await;
                            defmt::unimplemented!("finished pio test");
                            // let test = i2c_tests::simple_read::FP;
                            // run_fp_test(n, test, &mut ctx.channels.up, &mut peripherals).await;
                        }
                        HostToFPCommand::Run(n @ 2) => {
                            debug!("running test {}", n);
                            let test = i2c_tests::simple_write::FP;
                            run_fp_test(n, test, &mut ctx.channels.up, &mut peripherals).await;
                        }
                        HostToFPCommand::Run(_) => defmt::todo!(),
                    }
                    remaining
                }
            };
        }
    }
}
