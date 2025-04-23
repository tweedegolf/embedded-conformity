#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp::gpio;
use embassy_time::{Duration, Timer};
use gpio::{Level, Output};
use {panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    // Initialise Peripherals
    let p = embassy_rp::init(Default::default());

    // Create LED
    let mut led = Output::new(p.PIN_13, Level::Low);

    // Loop
    loop {
        // Turn LED On
        led.set_high();

        // Wait 100ms
        Timer::after(Duration::from_millis(100)).await;

        // Turn Led Off
        led.set_low();

        // Wait 100ms
        Timer::after(Duration::from_millis(100)).await;
    }
}