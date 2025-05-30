use serde::{Deserialize, Serialize};
use strum::EnumIter;

#[allow(non_camel_case_types)]
#[derive(EnumIter, Serialize, Deserialize)]
pub enum TestSelector {
    Sanity_Pin,
    I2C_SimpleRead,
    I2C_SimpleWrite,
}
