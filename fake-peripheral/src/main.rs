//! This example shows how to create a pwm using the PIO module in the RP2040 chip.

#![no_std]
#![no_main]

use defmt::info;
use embassy_executor::Spawner;
use embassy_rp::gpio::{Level, Output};
use test_suite::{postcard::accumulator::{CobsAccumulator, FeedResult}, protocol::HostToFP};

mod pio;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    // Even though we won't be running the test suite on this device, we will still import it for
    // some of the helper functions like `init`
    let mut ctx = test_suite::init();

    let p = embassy_rp::init(Default::default());

    let mut led = Output::new(p.PIN_13, Level::Low);
    led.set_high();

    let mut raw_buf = [0u8; 128];
    let mut cobs_buf: CobsAccumulator<256> = CobsAccumulator::new();

    // Continously read the RTT buffer and decode the messages
    loop {
        let ct = ctx.channels.down.read(&mut raw_buf);
        // Finished reading input
        if ct == 0 {
            continue;
        }

        let buf = &raw_buf[..ct];
        let mut window = &buf[..];

        'cobs: while !window.is_empty() {
            window = match cobs_buf.feed::<HostToFP>(&window) {
                FeedResult::Consumed => break 'cobs,
                FeedResult::OverFull(new_wind) => new_wind,
                FeedResult::DeserError(new_wind) => new_wind,
                FeedResult::Success { data, remaining } => {
                    // Do something with `data: MyData` here.
                    match data {
                        HostToFP::Init => info!("Init Ready"),
                        HostToFP::Run(_) => todo!(),
                    }

                    remaining
                }
            };
        }
    }
}


