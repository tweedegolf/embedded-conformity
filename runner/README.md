# Runner

Host-side orchestrator that compiles, flashes, and coordinates test execution between shield and DUT.

## CLI Commands

### `list`
Lists connected debug probes with serial numbers:
```bash
cargo run -- list
```

### `test`
Runs tests on configured devices:
```bash
# All tests
cargo run -- test --config config.toml

# Specific test
cargo run -- test -c config.toml -s I2C_SimpleWrite
```

### `example-config`
Prints example configuration:
```bash
cargo run -- example-config > my-config.toml
```

## Configuration
See config.toml

## Execution Flow

1. Parse configuration and find probes
2. Build firmware for both devices (respects `.cargo/config.toml`)
3. Flash via probe-rs
4. Reset and establish debug sessions
5. Initialize RTT channels and defmt logging
6. Coordinate test execution via RTT control messages
7. Report results

## Communication

Uses RTT channels for bidirectional communication:
- **Up Channel 0**: defmt logs
- **Up Channel 1**: Control messages (device → runner)
- **Down Channel 0**: Control commands (runner → device)

Messages use COBS framing with postcard serialization.

## Logging

Configure with `RUST_LOG`:
```bash
# Debug output
RUST_LOG=debug cargo run -- test

# Only runner logs
RUST_LOG=runner=debug cargo run -- test
```

Log prefixes:
- `runner::` - Runner operations
- `DUT:` - Device under test logs
- `FP:` / `Shield:` - Shield logs

## Troubleshooting

**Probe not found**: Check serial with `cargo run -- list`
**RTT attachment fails**: Verify firmware initializes RTT, sometimes debugger gets stuck so try power cycling
**Tests hang**: Check `RUST_LOG=debug` output, verify both devices respond

