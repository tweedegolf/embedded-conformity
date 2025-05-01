use heapless::Vec;
use postcard::{
    ser_flavors::{Cobs, HVec},
    serialize_with_flavor,
};
use rtt_target::UpChannel;
use serde::{Deserialize, Serialize};

#[cfg(feature = "std")]
extern crate alloc;

#[derive(Serialize, Deserialize)]
pub enum HostToDUT {
    /// Run a specific test
    Init,
    Run(u32),
}

#[derive(Serialize, Deserialize)]
pub enum DUTToHost {
    TestFailure(u32),
    Success(u32),
    Finished,
}

#[derive(Serialize, Deserialize)]
pub enum HostToFP {
    Init,
    Run(u32),
}

#[derive(Serialize, Deserialize)]
pub enum FPToHost {
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
