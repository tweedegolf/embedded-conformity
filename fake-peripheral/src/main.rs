//! This example shows how to create a pwm using the PIO module in the RP2040 chip.

#![no_std]
#![no_main]
use core::time::Duration;

use embassy_executor::Spawner;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::PIO0;
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_rp::pio_programs::pwm::{PioPwm, PioPwmProgram};
use embassy_rp::{Peripherals, bind_interrupts};
use embassy_time::Timer;
use panic_probe as _;
use rtt_target::{
    ChannelMode, DownChannel, rprintln, rtt_init, rtt_init_defmt, rtt_init_print, set_print_channel,
};

use defmt::info;

const REFRESH_INTERVAL: u64 = 20000;

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let mut channels = rtt_init! {
        up: {
            0: {
                size: 1024,
                mode: ChannelMode::NoBlockSkip,
                name: "Log"
            }
        }
        down: {
            0: {
                size: 1024,
                name: "Control"
            }
        }
    };
    set_print_channel(channels.up.0);

    let p = embassy_rp::init(Default::default());

    let mut led = Output::new(p.PIN_13, Level::Low);
    led.set_high();

    wait_for_host(&mut channels.down.0);

    loop {
        Timer::after_millis(100).await;
        led.toggle();
    }
}

fn wait_for_host(down: &mut DownChannel) {
    let mut read_buf = [0; 1024];
    let mut read;
    'outer: loop {
        read = down.read(&mut read_buf);
        for i in 0..read {
            if read_buf[i] == 42 {
                break 'outer;
            }
        }
    }
    rprintln!("Starting test");
}

async fn pio_example(p: Peripherals) {
    let Pio {
        mut common, sm0, ..
    } = Pio::new(p.PIO0, Irqs);

    let prg = PioPwmProgram::new(&mut common);
    let mut pwm_pio = PioPwm::new(&mut common, sm0, p.PIN_13, &prg);
    pwm_pio.set_period(Duration::from_micros(REFRESH_INTERVAL));
    pwm_pio.start();

    let mut duration = 0;
    loop {
        duration = (duration + 1) % 1000;
        pwm_pio.write(Duration::from_micros(duration));
        Timer::after_millis(1).await;
    }
}
