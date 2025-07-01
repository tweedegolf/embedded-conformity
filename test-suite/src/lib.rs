#![no_std]
#![allow(async_fn_in_trait)]

use defmt::Format;
use postcard::accumulator::{CobsAccumulator, FeedResult};
use rtt_target::{rtt_init, set_defmt_channel, set_print_channel, ChannelMode, DownChannel, UpChannel};

pub use heapless;
pub use postcard;
use serde::Deserialize;
pub use strum;

pub mod i2c_tests;
pub mod list_of_tests;
pub mod sanity_tests;

pub mod dut;
pub mod fp;

pub mod protocol;

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
                name: "defmt"
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

