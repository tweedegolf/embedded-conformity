# Shield

Fake peripheral firmware running on RP2040/RP2350 that emulates peripherals and validates DUT behavior.

## Code Structure

Shield firmware is minimal - most logic lives in `test-suite/`:

All test implementations are in `test-suite/src/i2c_tests/` with PIO programs in `test-suite/src/i2c_tests/pio_tests/`.
