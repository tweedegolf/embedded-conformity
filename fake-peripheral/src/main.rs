#![no_std]
#![no_main]

use defmt::info;
use embassy_executor::Spawner;
use embassy_rp::gpio::{Input, Level, Output};
use embassy_time::{Duration, with_timeout};
use panic_probe as _;
use test_suite::{
    postcard::accumulator::{CobsAccumulator, FeedResult},
    protocol::{FPToHost, HostToFP, HostToFPCommand, send_to_host},
};

mod pio;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    // Even though we won't be running the test suite on this device, we will still import it for
    // some of the helper functions like `init`
    let mut ctx = test_suite::init();

    let p = embassy_rp::init(Default::default());

    let mut led = Output::new(p.PIN_13, Level::Low);
    led.set_high();

    let mut input_one = Input::new(p.PIN_10, embassy_rp::gpio::Pull::None);

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
        let mut window = buf;

        'cobs: while !window.is_empty() {
            window = match cobs_buf.feed::<HostToFP>(window) {
                FeedResult::Consumed => break 'cobs,
                FeedResult::OverFull(new_wind) => new_wind,
                FeedResult::DeserError(new_wind) => new_wind,
                FeedResult::Success { data, remaining } => {
                    send_to_host(FPToHost::Ack(data.id), &mut ctx.channels.up);
                    // Do something with `data: MyData` here.
                    match data.command {
                        HostToFPCommand::Init => info!("Init Ready"),
                        HostToFPCommand::Run(0) => {
                            run_test(async {
                                test_one(&mut input_one).await;
                            })
                            .await;

                            send_to_host(FPToHost::Success(0), &mut ctx.channels.up);
                        }
                        HostToFPCommand::Run(_) => todo!(),
                    }

                    remaining
                }
            };
        }
    }
}

async fn run_test<F: Future>(fut: F) {
    let res = with_timeout(Duration::from_millis(5000), fut).await;

    res.unwrap();
}

async fn test_one(input: &mut Input<'_>) {
    input.wait_for_high().await;
}
