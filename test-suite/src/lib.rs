#![no_std]

use defmt::info;
use embedded_hal::i2c::{Error, I2c};
use rtt_target::{ChannelMode, DownChannel, UpChannel, rtt_init, set_defmt_channel};

pub const ADDR: u8 = 0x42;
pub struct BasicTest<I2C> {
    i2c: I2C,
}

impl<I2C: I2c> BasicTest<I2C> {
    pub fn new(i2c: I2C) -> Self {
        Self { i2c }
    }

    pub fn simple_write(&mut self) -> Result<(), I2C::Error> {
        let value = [0x76];
        self.i2c.write(ADDR, &value)?;
        Ok(())
    }
}

pub struct Channels {
    pub up: [UpChannel; 1],
    pub down: [DownChannel; 1],
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
            up: [channels.up.1],
            down: [channels.down.0],
        },
    }
}

pub const MAGIC_START_BYTE: u8 = 42;
    pub fn wait_for_host(down: &mut DownChannel) {
    let mut read_buf = [0; 32];
    let mut read;
    'outer: loop {
        read = down.read(&mut read_buf);
        for i in 0..read {
            if read_buf[i] == MAGIC_START_BYTE {
                break 'outer;
            }
        }
    }
}

pub fn run_tests(mut ctx: Context) {
    wait_for_host(&mut ctx.channels.down[0]);
    info!("DUT: Ready");
}
