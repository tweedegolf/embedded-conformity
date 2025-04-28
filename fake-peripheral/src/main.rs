//! This example shows how to create a pwm using the PIO module in the RP2040 chip.

#![no_std]
#![no_main]
use core::time::Duration;

use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::PIO0;
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_rp::pio_programs::pwm::{PioPwm, PioPwmProgram};
use embassy_time::Timer;
use rtt_target::{rprintln, rtt_init_defmt, rtt_init_print};
use panic_probe as _;

use defmt::info;

const REFRESH_INTERVAL: u64 = 20000;

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    // rtt_init_defmt!();
    rtt_init_print!();

    let p = embassy_rp::init(Default::default());
    let Pio { mut common, sm0, .. } = Pio::new(p.PIO0, Irqs);

    let mut led = Output::new(p.PIN_13, Level::Low);
    led.set_high();

    loop {
        rprintln!("Hello World");
        Timer::after_millis(100).await;
        led.toggle();
    }

    // // Note that PIN_25 is the led pin on the Pico
    // let prg = PioPwmProgram::new(&mut common);
    // let mut pwm_pio = PioPwm::new(&mut common, sm0, p.PIN_13, &prg);
    // pwm_pio.set_period(Duration::from_micros(REFRESH_INTERVAL));
    // pwm_pio.start();
    //
    // let mut duration = 0;
    // loop {
    //     duration = (duration + 1) % 1000;
    //     pwm_pio.write(Duration::from_micros(duration));
    //     Timer::after_millis(1).await;
    // }
}
