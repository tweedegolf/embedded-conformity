use heapless::Vec;
use postcard::{
    ser_flavors::{Cobs, HVec},
    serialize_with_flavor,
};

use rtt_target::UpChannel;
use serde::{Deserialize, Serialize};

#[cfg(feature = "std")]
extern crate alloc;
#[cfg(feature = "std")]
use rand::{RngCore, rngs::ThreadRng};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct HostToDUT {
    pub id: u32,
    pub command: HostToDUTCommand,
}

#[cfg(feature = "std")]
impl HostToDUT {
    pub fn new(command: HostToDUTCommand) -> Self {
        let mut rng = rand::rng();
        Self {
            id: rng.next_u32(),
            command,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum HostToDUTCommand {
    /// Run a specific test
    Init,
    Run(u32),
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum DUTToHost {
    Ack(u32),
    TestFailure(u32),
    Success(u32),
    Finished,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct HostToFP {
    pub id: u32,
    pub command: HostToFPCommand,
}

#[cfg(feature = "std")]
impl HostToFP {
    pub fn new(command: HostToFPCommand) -> Self {
        let mut rng = rand::rng();
        Self {
            id: rng.next_u32(),
            command,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum HostToFPCommand {
    Init,
    Run(u32),
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum FPToHost {
    Ack(u32),
    TestFailure(u32),
    Success(u32),
}

pub fn to_bytes<T: Serialize>(message: T) -> Vec<u8, 128> {
    serialize_with_flavor(&message, Cobs::try_new(HVec::<128>::new()).unwrap()).unwrap()
}

#[cfg(feature = "std")]
pub fn to_bytes_alloc<T: Serialize>(message: T) -> alloc::vec::Vec<u8> {
    postcard::to_allocvec_cobs(&message).unwrap()
}

pub fn send_to_host<T: Serialize>(msg: T, up: &mut UpChannel) {
    let bytes = to_bytes(msg);
    let mut rem = bytes.len();

    loop {
        let n = up.write(&bytes);
        rem -= n;

        if rem == 0 {
            break;
        }
    }
}
