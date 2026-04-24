# Embedded Conformity

A Hardware-in-the-Loop (HIL) testing suite for validating Rust's `embedded-hal` trait implementations across different microcontroller platforms.

## Overview

Embedded Conformity provides an automated testing framework that verifies embedded-hal implementations conform to the specification. The system uses real hardware to test I2C, GPIO, and other peripheral implementations, ensuring that drivers correctly implement the embedded-hal abstraction layer.

## Architecture

The system consists of three main components:

1. **Runner** (Host): Orchestrates test execution, compiles and flashes firmware, monitors test results
2. **Shield** (Fake Peripheral): RP2350/RP2040 microcontroller that emulates peripherals and generates test inputs
3. **DUT** (Device Under Test): The microcontroller being tested, running the embedded-hal implementation

```
┌───────────────────────────────────────────────────────────┐
│                        Host (Runner)                      │
│  - Compiles & flashes firmware                            │
│  - Orchestrates test execution                            │
│  - Monitors via RTT/defmt                                 │
│  - Reports test results                                   │
└────────┬─────────────────────────────────────┬────────────┘
         │ USB/Debugger                        │ USB/Debugger
         │                                     │
┌────────▼────────────┐          ┌─────────────▼─────────────┐
│   Shield (RP2350)   │◄────────►│      DUT (e.g. nRF52)     │
│  - I2C peripheral   │  PINS    │  - embedded-hal impl      │
│  - PIO test logic   │          │  - Runs test suite        │
│  - Async executor   │          │  - Reports via RTT        │
└─────────────────────┘          └───────────────────────────┘
```

## Nomenclature

- **DUT**: Device Under Test - the microcontroller being tested (e.g., nRF52, STM32, ESP32-C6)
- **Shield**: or Fake peripheral - an RP2350/RP2040 that emulates peripherals to generate test inputs
- **Runner**: Host application that orchestrates the entire test process
- **RTT**: Real-Time Transfer - protocol for fast communication between chip and debugger
- **defmt**: Deferred formatting - efficient logging protocol for embedded devices
- **PIO**: Programmable I/O - RP2350/RP2040 hardware peripheral used for Programmable I/O operations

## Project Structure

```
embedded-conformity/
├── runner/          # Host application
│   ├── src/
│   │   ├── main.rs           # CLI interface
│   │   ├── coordinator.rs    # Test orchestration
│   │   └── defmt_logger.rs   # Logging infrastructure
│   ├── config.toml           # Device configuration (nRF52 example)
│   ├── esp.toml              # Device configuration (ESP32 example)
│   └── stm32.toml            # Device configuration (STM32 example)
│
├── test-suite/      # Shared test definitions (no_std)
│   ├── src/
│   │   ├── dut.rs            # DUT-side test framework
│   │   ├── fp.rs             # Shield-side test framework
│   │   ├── i2c_tests/        # I2C conformance tests
│   │   ├── sanity_tests/     # Basic GPIO tests
│   │   └── protocol.rs       # Communication protocol
│   └── Cargo.toml
│
├── shield/          # Shield firmware (RP2350/RP2040)
│   ├── src/main.rs
│   └── Cargo.toml
│
├── nRF52/           # DUT implementation (nRF52)
├── stm32/           # DUT implementation (STM32)
├── esp32c6/         # DUT implementation (ESP32-C6)
│
└── spec.md          # embedded-hal specification requirements
```

## Requirements

### Hardware

- **Shield**: A regular RP2040/RP2350 or special developed shield
- **DUT**: Any microcontroller with:
  - embedded-hal driver implementation
  - Debugger supported by probe-rs (SWD/JTAG)
- **Debuggers**: Two debug probes (one for shield, one for DUT)
- **Connections**: Wire GPIO and I2C pins between shield and DUT

### Software

- Rust toolchain (stable or nightly)
- probe-rs for flashing and debugging
- Target-specific compilation tools (see individual DUT directories)

## Getting Started

### 1. Hardware Setup

Connect the shield and DUT:
- I2C: SDA and SCL pins between devices
- GPIO: Test pins as defined in firmware
- Connect both devices to host via debuggers

### 2. Configuration

Create or modify a configuration file (e.g., `runner/config.toml`):

```toml
[device_under_test]
firmware_path = "../nRF52/"
serial = "001050295885"        # Debug probe serial number
chip = "nRF52805_xxAA"

[fake_peripheral]
firmware_path = "../shield/"
serial = "E6614103E78B5024"    # Debug probe serial number
chip = "rp2040"                # or "rp2350"
```

You can find debug probe serial numbers using `list` subcommand of the runner.

### 3. Running Tests

```bash
cd runner

# List available devices
cargo run -- list

# Run all tests
cargo run -- test --config config.toml

# Run specific test
cargo run -- test --config config.toml --selector I2C_SimpleWrite
```

## Available Tests

### Sanity Tests
- `Sanity_Pin`: Basic GPIO output functionality

### I2C Tests
- `I2C_SimpleRead`: Basic I2C read operation
- `I2C_SimpleWrite`: Basic I2C write operation
- `I2C_MultiWrite`: Multiple consecutive write operations
- `I2C_AddressNAK`: Address NACK error handling
- `I2C_DataNAK`: Data NACK error handling

## Adding a New Device

To add support for a new microcontroller:

1. Create a new directory for your device (e.g., `my-device/`)
2. Add `test-suite` as a dependency with the `dut` feature
3. Implement a minimal main.rs that:
   - Initializes your device's peripherals
   - Creates a `DutPeripherals` struct with I2C and GPIO
   - Calls `run_dut_tests()`

Example:

```rust
#![no_std]
#![no_main]

use test_suite::dut::{DutPeripherals, run_dut_tests};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let ctx = test_suite::init();
    let p = my_hal::init(Default::default());

    let peripherals = DutPeripherals {
        i2c: p.i2c0,      // Your I2C peripheral
        pin: p.gpio_a0,   // Your GPIO pin
    };

    run_dut_tests(ctx, peripherals);

    loop { /* wait */ } // Test suite finished
}
```

See `nRF52/`, `stm32/`, or `esp32c6/` directories for complete implementations.

## Logging Infrastructure

The project ensures all logs are visible to the runner:

- **Device → Debugger**: defmt with RTT for efficient, formatted logging
- **Debugger → Host**: probe-rs RTT channels
- **Host**: tracing for unified log output

Each log message includes file and line number information for debugging

## Testing Approach

Tests follow the embedded-hal specification:

1. **Test Definition**: Each test is defined in `test-suite/` with both DUT and Shield sides in the same file
2. **Coordination**: Runner sends commands to both devices via RTT
3. **Execution**: DUT performs operations while Shield validates behavior
4. **Reporting**: Results are sent back to runner via RTT and displayed

The architecture leverages Rust's generics so device-specific code is minimal - most test logic is shared.

## References

- [embedded-hal](https://docs.rs/embedded-hal/): The Hardware Abstraction Layer traits
- [probe-rs](https://probe.rs/): Embedded debugging toolkit
- [defmt](https://defmt.ferrous-systems.com/): Efficient embedded logging
- [embassy](https://embassy.dev/): Async embedded framework

## License

Copyright 2026 Tweede Golf B.V.

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
