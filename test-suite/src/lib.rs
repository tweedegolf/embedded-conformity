#![no_std]

use defmt::{Format, debug, error, trace, unwrap};
use embedded_hal::{digital::OutputPin, i2c::I2c};
use postcard::accumulator::{CobsAccumulator, FeedResult};
use protocol::{
    DUTToHost, FPToHost, HostToDUT, HostToDUTCommand, HostToFP, HostToFPCommand, send_to_host,
};
use rtt_target::{ChannelMode, DownChannel, UpChannel, rtt_init, set_defmt_channel};

pub use postcard;
use serde::Deserialize;

pub mod i2c_tests;
pub mod sanity_tests;

pub mod protocol;

#[cfg(feature = "fp")]
pub use embassy_rp;
#[cfg(feature = "fp")]
use embassy_rp::{gpio::Input, i2c::Instance, i2c_slave::I2cSlave};

pub struct Channels {
    pub up: UpChannel,
    pub down: DownChannel,
}

pub struct Context {
    pub channels: Channels,
}

pub fn init() -> Context {
    let channels = rtt_init! {
        up: {
            0: {
                size: 1024,
                mode: ChannelMode::NoBlockSkip, // TODO: probably different mode?
                name: "Log"
            }
            1: {
                size: 1024,
                mode: ChannelMode::NoBlockSkip,
                name: "Control"
            }
        }
        down: {
            0: {
                size: 1024,
                name: "Control"
            }
        }
    };

    set_defmt_channel(channels.up.0);

    Context {
        channels: Channels {
            up: channels.up.1,
            down: channels.down.0,
        },
    }
}

pub const NUM_TESTS: u32 = 3;

fn read_cobs<T: for<'de> Deserialize<'de>>(down: &mut DownChannel, mut fun: impl FnMut(T)) -> ! {
    let mut raw_buf = [0u8; 128];
    let mut cobs_buf: CobsAccumulator<256> = CobsAccumulator::new();

    loop {
        let ct = down.read(&mut raw_buf);
        // Finished reading input
        if ct == 0 {
            continue;
        }

        let buf = &raw_buf[..ct];
        let mut window = buf;

        'cobs: while !window.is_empty() {
            window = match cobs_buf.feed::<T>(window) {
                FeedResult::Consumed => break 'cobs,
                FeedResult::OverFull(new_wind) => new_wind,
                FeedResult::DeserError(new_wind) => new_wind,
                FeedResult::Success { data, remaining } => {
                    fun(data);
                    remaining
                }
            };
        }
    }
}

pub fn run_dut_tests<P: OutputPin, I2C: I2c>(mut ctx: Context, mut session: Session<I2C, P>)
where
    <I2C as embedded_hal::i2c::ErrorType>::Error: defmt::Format,
    <P as embedded_hal::digital::ErrorType>::Error: defmt::Format,
{
    read_cobs(&mut ctx.channels.down, |data: HostToDUT| {
        send_to_host(DUTToHost::Ack(data.id), &mut ctx.channels.up);

        match data.command {
            HostToDUTCommand::Init => {}
            HostToDUTCommand::Run(n @ 0) => {
                debug!("running test {}", n);
                let test = sanity_tests::pin_test::Dut;
                run_dut_test(n, test, &mut ctx.channels.up, &mut session);
            }
            HostToDUTCommand::Run(n @ 1) => {
                debug!("running test {}", n);
                let test = i2c_tests::simple_read::Dut;
                run_dut_test(n, test, &mut ctx.channels.up, &mut session);
            }
            HostToDUTCommand::Run(n @ 2) => {
                debug!("running test {}", n);
                let test = i2c_tests::simple_write::Dut;
                run_dut_test(n, test, &mut ctx.channels.up, &mut session);
            }
            HostToDUTCommand::Run(_) => defmt::todo!(),
        }
    });
}

#[cfg(feature = "fp")]
pub async fn run_fp_tests<I: Instance>(
    mut ctx: Context,
    inp: &mut Input<'_>,
    i2c_target: &mut I2cSlave<'_, I>,
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
                        HostToFPCommand::Run(n @ 0) => {
                            debug!("running test {}", n);
                            let test = sanity_tests::pin_test::FP(inp);
                            run_fp_test(n, test, &mut ctx.channels.up).await;
                        }
                        HostToFPCommand::Run(n @ 1) => {
                            debug!("running test {}", n);
                            let test = i2c_tests::simple_read::FP(i2c_target);
                            run_fp_test(n, test, &mut ctx.channels.up).await;
                        }
                        HostToFPCommand::Run(n @ 2) => {
                            debug!("running test {}", n);
                            let test = i2c_tests::simple_write::FP(i2c_target);
                            run_fp_test(n, test, &mut ctx.channels.up).await;
                        }
                        HostToFPCommand::Run(_) => defmt::todo!(),
                    }
                    remaining
                }
            };
        }
    }
}

fn run_dut_test<I2C: I2c, P: OutputPin>(n: u32, mut test: impl DutTest<I2C, P>, up: &mut UpChannel, session: &mut Session<I2C, P>) {
    if let Err(e) = test.setup(session) {
        error!("Encountered error during setup of test {}: {:?}", n, &e);
        send_to_host(DUTToHost::TestFailure(n), up);
        return;
    }

    if let Err(e) = test.run(session) {
        error!("Encountered error during run of test {}: {:?}", n, &e);
        send_to_host(DUTToHost::TestFailure(n), up);
        return;
    }

    send_to_host(DUTToHost::Success(n), up);

    // we crash as we can not guarantee to correctness of the system
    unwrap!(test.teardown(session))
}

#[cfg(feature = "fp")]
async fn run_fp_test(n: u32, mut test: impl FPTest, up: &mut UpChannel) {
    if let Err(e) = test.setup().await {
        error!("Encountered error during setup of test {}: {:?}", n, &e);
        send_to_host(FPToHost::TestFailure(n), up);
        return;
    }

    if let Err(e) = test.run().await {
        error!("Encountered error during run of test {}: {:?}", n, &e);
        send_to_host(FPToHost::TestFailure(n), up);
        return;
    }

    send_to_host(FPToHost::Success(n), up);

    // we crash as we can not guarantee to correctness of the system
    unwrap!(test.teardown().await)
}

pub struct Session<I2C: I2c, P: OutputPin> {
    pub i2c: I2C,
    pub pin: P,
}

trait DutTest<I2C: I2c, P: OutputPin> {
    type E: Format;

    fn setup(&mut self, session: &mut Session<I2C, P>) -> Result<(), Self::E>;
    fn run(&mut self, session: &mut Session<I2C, P>) -> Result<(), Self::E>;
    fn teardown(&mut self, session: &mut Session<I2C, P>) -> Result<(), Self::E>;
}

trait FPTest {
    type E: Format;

    async fn setup(&mut self) -> Result<(), Self::E>;
    async fn run(&mut self) -> Result<(), Self::E>;
    async fn teardown(&mut self) -> Result<(), Self::E>;
}
