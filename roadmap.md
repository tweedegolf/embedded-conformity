# More tests:

## `embedded-hal(-async)`

* Digital (in progress)
  * [`InputPin`](https://docs.rs/embedded-hal/latest/embedded_hal/digital/trait.InputPin.html)
  * [`OutputPin`](https://docs.rs/embedded-hal/latest/embedded_hal/digital/trait.OutputPin.html)
  * [`StatefulOutputPin`](https://docs.rs/embedded-hal/latest/embedded_hal/digital/trait.StatefulOutputPin.html)
  * [`Wait`](https://docs.rs/embedded-hal-async/latest/embedded_hal_async/digital/trait.Wait.html)
* I2C (in progress)
  * [`I2c`](https://docs.rs/embedded-hal/latest/embedded_hal/i2c/trait.I2c.html)
  * Arbitrary [`Operation`](https://docs.rs/embedded-hal/latest/embedded_hal/i2c/enum.Operation.html) sequences
  * Both [`AddressMode`s](https://docs.rs/embedded-hal/latest/embedded_hal/i2c/trait.AddressMode.html)
  * All [`ErrorKind`s](https://docs.rs/embedded-hal/latest/embedded_hal/i2c/enum.ErrorKind.html)
  * Different clock rates
  * Bus arbitration
* [`delay::DelayNs`](https://docs.rs/embedded-hal/latest/embedded_hal/delay/trait.DelayNs.html)
* [`pwm::SetDutyCycle`](https://docs.rs/embedded-hal/latest/embedded_hal/pwm/trait.SetDutyCycle.html)
* SPI
  * [`SpiBus`](https://docs.rs/embedded-hal/latest/embedded_hal/spi/trait.SpiBus.html)
  * [`SpiDevice`](https://docs.rs/embedded-hal/latest/embedded_hal/spi/trait.SpiDevice.html)
    * [CS-to-clock delays](https://docs.rs/embedded-hal/latest/embedded_hal/spi/index.html#cs-to-clock-delays)
    * [`Operation::DelayNs`](https://docs.rs/embedded-hal/latest/embedded_hal/spi/enum.Operation.html#variant.DelayNs)
  * Different clock rates
  * Different word sizes
* For `async` methods: Test if the Wakers are set correctly

## Not covered by `trait`s

* ADC
* DAC
* Analog Comparators
* Pull-Up / Pull-Down
* QEI
* Timer Capture
* Timer Triggered ADC
* Capacitive Touch
* Sleep modes
  * Power Usage
  * Wake Up Sources
* Accelerators
  * DMA (Memory-to-Memory)
  * Hashing
  * Crypto
  * Cordic
  * FMAC
  * JPEG
* CRC units
* RNG
* I2S
* PDM
* RTC
* I3C (?)
* I2C Target

## [`smart-leds-trait`](https://docs.rs/smart-leds-trait/latest/smart_leds_trait/)

* Timing

## UART ([`embedded_io`](https://docs.rs/embedded-io/latest/embedded_io/))

* Baud rate calculation
* Parity
* Word sizes
* Start/Stop bit lengths
* Flow control
* Line break detection
* Error conditions:
  * Framing
  * Parity
  * Break characters
  * FIFO overrun
  * Noise
* Does the connection stay in sync over long running sequences

## Storage Traits

* Flash
* Block device

## Memory

* Memory ECC
* MPU
* Trustzone

## Interfaces that need extra hardware

* [`embedded_can`](https://docs.rs/embedded-can/latest/embedded_can/)
  * Data rate calculation
  * Receive filters
* USB ([`embassy_usb_driver`](https://docs.rs/embassy-usb-driver/latest/embassy_usb_driver/))
* USB Host
* USB-C PD
* Ethernet
* NFC
* Radio
  * Wifi
  * IEEE 802.15.4
  * Bluetooth

# More Devices

* Currently supported
  * ESP32-C6
  * NRF52840
  * STM32L476RG

* Easy to add (mostly embassy HALs)
  * RP2040 / RP2350
  * STM32 with Nucleo-64 or Nucleo-144 board ([list](https://www.st.com/en/evaluation-tools/stm32-nucleo-boards.html))
  * Nordic nRF chips (except nRf54L series)
  * `embassy-nxp`: LPC55, mimxrt1011 (Metro M7), mimxrt1062
  * `embassy-imxrt`: MIMXRT685S
  * [`atsamd`](https://github.com/atsamd-rs/atsamd) (Metro M0, Metro M4)

* Need adapter boards
  * ESP32-*
  * MSPM0
  * [neorv32](https://github.com/kurtjd/embassy-neorv32) (FPGA based)
  * Adafruit Seasaw
  * Beaglebone
  * RPi 1-5
  * [MSP430](https://docs.rs/msp430fr2x5x-hal/latest/msp430fr2x5x_hal/)
  * Nordic nRf54L series
  * [PIC32](https://github.com/kiffie/pic32-rs)

# Improving the test runner

* Running with different pin assignments
* Generating better reports
* Synchronization barriers
* Integrate it into CI (TODO colaborate with Systemscape?)
