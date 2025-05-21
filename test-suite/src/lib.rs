#![no_std]

use core::fmt;

use defmt::{Debug2Format, Format, error, info, unwrap};
use embedded_hal::{digital::OutputPin, i2c::I2c};
use postcard::accumulator::{CobsAccumulator, FeedResult};
use protocol::{
    DUTToHost, FPToHost, HostToDUT, HostToDUTCommand, HostToFP, HostToFPCommand, send_to_host,
};
use rtt_target::{ChannelMode, DownChannel, UpChannel, rtt_init, set_defmt_channel};

pub use postcard;
use sanity_tests::*;
use serde::Deserialize;

pub mod i2c_tests;
pub mod protocol;
mod sanity_tests;
use i2c_tests::i2c_test_simple;

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

pub const NUM_TESTS: u32 = 2;

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

pub fn run_dut_tests<T: OutputPin, I2C: I2c>(mut ctx: Context, output: &mut T, i2c: &mut I2C)
where
    <I2C as embedded_hal::i2c::ErrorType>::Error: defmt::Format,
    <T as embedded_hal::digital::ErrorType>::Error: defmt::Format,
{
    read_cobs(&mut ctx.channels.down, |data: HostToDUT| {
        send_to_host(DUTToHost::Ack(data.id), &mut ctx.channels.up);

        match data.command {
            HostToDUTCommand::Init => {}
            HostToDUTCommand::Run(n @ 0) => {
                let t = pin_test::Dut(output);
                run_dut_test(n, t, &mut ctx.channels.up);
            }
            HostToDUTCommand::Run(n @ 1) => {
                let test = i2c_test_simple::Dut(i2c);
                run_dut_test(n, test, &mut ctx.channels.up);
            }
            HostToDUTCommand::Run(_) => todo!(),
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
                            let test = pin_test::FP(inp);
                            run_fp_test(n, test, &mut ctx.channels.up).await;
                            send_to_host(FPToHost::Success(n), &mut ctx.channels.up);
                        }
                        HostToFPCommand::Run(n @ 1) => {
                            let test = i2c_test_simple::FP(i2c_target);
                            run_fp_test(n, test, &mut ctx.channels.up).await;
                            send_to_host(FPToHost::Success(n), &mut ctx.channels.up);
                        }
                        HostToFPCommand::Run(_) => unimplemented!(),
                    }
                    remaining
                }
            };
        }
    }
}

fn run_dut_test(n: u32, mut test: impl DutTest, up: &mut UpChannel) {
    if let Err(e) = test.setup() {
        error!("Encountered error during setup of test {}: {:?}", n, &e);
        send_to_host(DUTToHost::TestFailure(n), up);
        return;
    }

    if let Err(e) = test.run() {
        error!("Encountered error during run of test {}: {:?}", n, &e);
        send_to_host(DUTToHost::TestFailure(n), up);
        return;
    }

    send_to_host(DUTToHost::Success(n), up);

    // we crash as we can not guarantee to correctness of the system
    unwrap!(test.teardown())
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

    // we crash as we can not guarantee to correctness of the system
    unwrap!(test.teardown().await)
}

trait DutTest {
    type E: Format;

    fn setup(&mut self) -> Result<(), Self::E>;
    fn run(&mut self) -> Result<(), Self::E>;
    fn teardown(&mut self) -> Result<(), Self::E>;
}

trait FPTest {
    type E: Format;

    async fn setup(&mut self) -> Result<(), Self::E>;
    async fn run(&mut self) -> Result<(), Self::E>;
    async fn teardown(&mut self) -> Result<(), Self::E>;
}
