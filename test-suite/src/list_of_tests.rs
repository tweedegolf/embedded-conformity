use defmt::Format;
use serde::{Deserialize, Serialize};
use strum::EnumIter;

/// When adding a test case an entry MUST also be added here
#[allow(non_camel_case_types)]
#[derive(EnumIter, Serialize, Deserialize, Debug, Format, PartialEq, Eq, Clone, Copy)]
pub enum TestSelector {
    Sanity_Pin,
    I2C_SimpleRead,
    I2C_SimpleWrite,
    I2C_MultiWrite,
}
