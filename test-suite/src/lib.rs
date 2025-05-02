#![no_std]

use defmt::info;
use embedded_hal::{
    digital::OutputPin,
    i2c::{Error, I2c},
};
use postcard::accumulator::{CobsAccumulator, FeedResult};
use protocol::{DUTToHost, HostToDUT, HostToDUTCommand, send_to_host};
use rtt_target::{ChannelMode, DownChannel, UpChannel, rtt_init, set_defmt_channel};

pub use postcard;
use sanity_tests::test_one;

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
        let mut window = &buf[..];

        'cobs: while !window.is_empty() {
            window = match cobs_buf.feed::<HostToDUT>(&window) {
                FeedResult::Consumed => break 'cobs,
                FeedResult::OverFull(new_wind) => new_wind,
                FeedResult::DeserError(new_wind) => new_wind,
                FeedResult::Success { data, remaining } => {
                    // Send ack
                    send_to_host(DUTToHost::Ack(data.id), &mut ctx.channels.up);

                    match data.command {
                        HostToDUTCommand::Init => info!("Init Ready"),
                        HostToDUTCommand::Run(0) => test_one(output).unwrap(),
                        HostToDUTCommand::Run(_) => todo!(),
                    }

                    remaining
                }
            };
        }
    }
}
