#![cfg(feature = "fp")]

use defmt::{Format, assert_eq, debug, intern, todo, trace, unwrap};
use embassy_futures::select::{self, Either4, select4};
use embassy_rp::gpio::Pull;
use embassy_rp::pio::program::ProgramWithDefines;
use embassy_rp::pio::{self, Direction, ShiftConfig, ShiftDirection};
use embassy_rp::pio::{Config, Pio, program::pio_file};
use heapless::{Deque, Vec};

struct I2CTesterPIO<'a, 'd, I: pio::Instance> {
    pio: &'a mut Pio<'d, I>,
    sda: &'a mut pio::Pin<'d, I>,
    scl: &'a mut pio::Pin<'d, I>,
}

impl<'a, 'd, I: pio::Instance> I2CTesterPIO<'a, 'd, I> {
    fn new(
        pio: &'a mut Pio<'d, I>,
        sda: &'a mut pio::Pin<'d, I>,
        scl: &'a mut pio::Pin<'d, I>,
    ) -> I2CTesterPIO<'a, 'd, I> {
        let program = pio_file!("src/i2c_tests/i2c_slave.pio", select_program("i2c_slave"));
        let program = pio.common.load_program(&program.program);

        debug!("loaded");

        // I2C is Pull-Up
        sda.set_pull(Pull::Up);
        sda.set_pull(Pull::Up);

        // Config
        let mut config = Config::<I>::default();
        config.set_in_pins(&[sda, scl]);
        config.set_out_pins(&[sda]);
        config.set_set_pins(&[sda]);
        config.set_jmp_pin(sda);
        config.use_program(&program, &[sda]);
        config.shift_in = ShiftConfig {
            threshold: 8,
            direction: ShiftDirection::Left,
            auto_fill: true,
        };
        config.shift_out = ShiftConfig {
            threshold: 8,
            direction: ShiftDirection::Left,
            auto_fill: true,
        };

        // Assign this Config+Program to sm0
        pio.sm0.set_config(&config);
        pio.sm0.set_pin_dirs(Direction::In, &[sda, scl]);

        Self { pio, sda, scl }
    }

    async fn run(&mut self) {
        // Ideas:
        // - use wait_pull and wait_push similarly as the https://docs.rs/embassy-rp/latest/src/embassy_rp/pio_programs/uart.rs.html
        // - Use IRQs to signal certain states of the program, verify against expectations
        //      - IRQ0 = Start + Address
        //      - IRQ1 = Read
        //      - IRQ2 = Write
        //      - IRQ3 = Program End

        self.pio.sm0.tx().push(13u32.to_be());

        self.pio.sm0.set_enable(true);
    }
}

pub async fn test_pio_i2c_slave<'a, I: embassy_rp::pio::Instance>(
    pio: &mut Pio<'a, I>,
    sda: &mut embassy_rp::pio::Pin<'a, I>,
    scl: &mut embassy_rp::pio::Pin<'a, I>,
) {
    let program = pio_file!("src/i2c_tests/i2c_slave.pio", select_program("i2c_slave"));
    let a = program.public_defines;
    let program = pio.common.load_program(&program.program);

    // i2c requires pull-up
    sda.set_pull(Pull::Up);
    scl.set_pull(Pull::Up);

    let mut config = Config::<I>::default();
    config.set_in_pins(&[sda, scl]);
    config.set_out_pins(&[sda]);
    config.set_set_pins(&[sda]);
    config.set_jmp_pin(sda);
    config.use_program(&program, &[sda]);
    config.shift_in = ShiftConfig {
        threshold: 8,
        direction: ShiftDirection::Left,
        auto_fill: true,
    };
    config.shift_out = ShiftConfig {
        threshold: 8,
        direction: ShiftDirection::Left,
        auto_fill: true,
    };

    pio.sm0.set_config(&config);
    pio.sm0.set_pin_dirs(Direction::In, &[sda, scl]);

    pio.sm0.tx().push(13u32.to_be());

    pio.sm0.set_enable(true); // Start the state machine
    //
    //

    loop {
        match select4(
            pio.irq0.wait(),
            pio.irq1.wait(),
            pio.irq2.wait(),
            pio.irq3.wait(),
        )
        .await
        {
            //
            Either4::First(_) => {
                let rx = pio.sm0.rx();
                let data = rx.pull().to_be_bytes()[3];

                let address = data >> 1;
                let mode = data & 1 == 1; // true is read, false is write

                debug!(
                    "I2C Start, Address: 0x{:X}, Mode: {};",
                    address,
                    if mode {
                        intern!("Read")
                    } else {
                        intern!("Write")
                    }
                );
            }
            Either4::Second(_) => debug!("PIO I2C Read"),
            Either4::Third(_) => debug!("PIO I2C Write"),
            Either4::Fourth(_) => break,
        }
    }
    //
    // pio.irq0.wait().await; // Wait for the IRQ to be triggered
    // let rx = pio.sm0.rx();
    // let data = rx.pull().to_be_bytes()[3];
    //
    // let address = data >> 1;
    // let mode = data & 1 == 1; // true is read, false is write
    //
    // debug!(
    //     "I2C Start, Address: 0x{:X}, Mode: {}",
    //     address,
    //     if mode {
    //         intern!("Read")
    //     } else {
    //         intern!("Write")
    //     }
    // );
    //
    // pio.irq1.wait().await;
    // debug!("PIO Read");
    //
    // pio.irq3.wait().await;
}
