use probe_rs::probe::{list::Lister, Probe};
use probe_rs::Permissions;

fn main() {
    let lister = Lister::new();
    let probes = lister.list_all();

    dbg!(probes);

    // 1. Upload firmware to DUT
    // 2. Upload firmware to client (RP2040)
    // 3. Start Tests
    // 4. Report Status
}
