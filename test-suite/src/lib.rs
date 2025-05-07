#![no_std]

use core::fmt;

use defmt::{error, info, Debug2Format};
use embedded_hal::digital::OutputPin;
use postcard::accumulator::{CobsAccumulator, FeedResult};
use protocol::{DUTToHost, HostToDUT, HostToDUTCommand, send_to_host};
use rtt_target::{ChannelMode, DownChannel, UpChannel, rtt_init, set_defmt_channel};

pub use postcard;
use sanity_tests::TestOne;

mod i2c_tests;
pub mod protocol;
mod sanity_tests;

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

pub const NUM_TESTS: u32 = 1;

pub fn run_tests<T: OutputPin>(mut ctx: Context, output: &mut T) {
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
            window = match cobs_buf.feed::<HostToDUT>(window) {
                FeedResult::Consumed => break 'cobs,
                FeedResult::OverFull(new_wind) => new_wind,
                FeedResult::DeserError(new_wind) => new_wind,
                FeedResult::Success { data, remaining } => {
                    // Send ack
                    send_to_host(DUTToHost::Ack(data.id), &mut ctx.channels.up);

                    match data.command {
                        HostToDUTCommand::Init => info!("Init Ready"),
                        HostToDUTCommand::Run(n@0) => {
                            let t = TestOne(output);
                            run_test(n, t, &mut ctx.channels.up);
                        }
                        HostToDUTCommand::Run(_) => todo!(),
                    }

                    remaining
                }
            };
        }
    }
}

fn run_test(n: u32, mut test: impl Test, up: &mut UpChannel) {
    // TODO: Timeout, maybe from host side instead?
    if let Err(e) = test.setup() {
        error!("Encountered error during setup of test {}: {:?}", n, Debug2Format(&e));
        send_to_host(DUTToHost::TestFailure(n), up);
        return;
    }

    if let Err(e) = test.run() {
        error!("Encountered error during run of test {}: {:?}", n, Debug2Format(&e));
        send_to_host(DUTToHost::TestFailure(n), up);
        return;
    }

    send_to_host(DUTToHost::Success(n), up);

    test.teardown().unwrap();
}

// TODO: Test Harness
// Each test should
// - Have some preamble/setup
// - the actual test + timeout
// - some postamble/teardown
// - Communicate these states properly without copying too much code

trait Test {
    type E: fmt::Debug;

    fn setup(&mut self) -> Result<(), Self::E>;
    fn run(&mut self) -> Result<(), Self::E>;
    fn teardown(&mut self) -> Result<(), Self::E>;
}
