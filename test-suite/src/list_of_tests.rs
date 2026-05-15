use defmt::Format;
use serde::{Deserialize, Serialize};
use strum::{EnumIter, EnumString};

/// When adding a test case an entry MUST also be added here
#[allow(non_camel_case_types)]
#[derive(
    EnumIter, Serialize, Deserialize, Debug, Format, PartialEq, Eq, Clone, Copy, EnumString,
)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
pub enum TestSelector {
    Sanity_Pin,
    I2C_SimpleRead,
    I2C_SimpleWrite,
    I2C_MultiWrite,
    I2C_AddressNAK,
    I2C_DataNAK,
}
