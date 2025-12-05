# Test Suite

Shared `no_std` library containing all test definitions. Both DUT and shield import this crate.

## Design

Tests are defined once with both sides in the same file:
- **DUT side**: Performs embedded-hal operations
- **Shield side**: Validates the behavior

This approach keeps test logic together and allows sharing constants and logic between both sides.

## Structure

```
src/
├── lib.rs              # RTT initialization, common types
├── protocol.rs         # Communication protocol (HostToDUT, FPToHost, etc.)
├── list_of_tests.rs    # TestSelector enum (registry of all tests)
├── dut.rs              # DUT test framework (DutTest trait, run_dut_tests)
├── fp.rs               # Shield test framework (FPTest trait, run_fp_tests)
├── sanity_tests/       # Basic GPIO tests
└── i2c_tests/          # I2C conformance tests
    ├── simple_write.rs # DUT writes, shield validates
    ├── ...
    └── pio_tests/      # PIO programs
```

## Writing a Test

Each test file defines both DUT and shield behavior:

```rust
// Test constant shared between both sides
const PAYLOAD: u8 = 13;

pub struct I2C_SimpleWrite;

// DUT side: perform the operation
impl<P: OutputPin, T: I2c> DutTest<T, P> for I2C_SimpleWrite {
    const S: TestSelector = TestSelector::I2C_SimpleWrite;

    fn run(&mut self, session: &mut DutPeripherals<T, P>) -> Result<(), TestError> {
        session.i2c.write(I2C_DEFAULT_ADDRESS, &[PAYLOAD])?;
        Ok(())
    }
}

// Shield side: validate the operation
#[cfg(feature = "fp")]
impl<I: i2c::Instance, P: pio::Instance> FPTest<I, P> for I2C_SimpleWrite {
    const S: TestSelector = TestSelector::I2C_SimpleWrite;

    async fn run(&mut self, peripherals: &mut FPPeripherals<'_, I, P>) -> Result<(), ()> {
        I2cSlaveTester::new(&mut peripherals.i2c)
            .expect_write(&[PAYLOAD])
            .run()
            .await?;
        Ok(())
    }
}
```

## Adding a New Test

1. **Create test file** in appropriate directory (e.g., `i2c_tests/my_test.rs`)

2. **Add to TestSelector** in `list_of_tests.rs`:
```rust
pub enum TestSelector {
    // ...
    I2C_MyTest,
}
```

3. **Implement DutTest** and **FPTest** traits

4. **Register in executors**:
   - Add match arm in `dut.rs::run_dut_tests()`
   - Add match arm in `fp.rs::run_fp_tests()`

## Features

- `std`: Enable for host (runner) usage
- `dut`: Enable for device under test
- `fp`: Enable for shield (fake peripheral)

## Protocol

Communication uses postcard serialization with COBS framing over RTT, see ./src/protocol.rs

## Logging

Use `defmt` macros for logging:
```rust
use defmt::{info, debug, error};

info!("Test starting");
debug!("Value: {}", x);
error!("Test failed: {}", reason);
```

Logs automatically include file and line number information and are shown on the runner

