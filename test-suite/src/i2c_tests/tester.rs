use defmt::{Format, assert, error, unwrap, warn};
use embassy_rp::{i2c::Instance, i2c_slave::I2cSlave};
use heapless::{Deque, Vec};

pub const NUM_RESPONSE: usize = 8;
pub const BUFFER_SIZE: usize = 32;

pub struct I2cSlaveTester<'a, 'b, I: Instance> {
    slave: &'b mut I2cSlave<'a, I>,
    read_responses: Deque<&'b [u8], NUM_RESPONSE>,
    write_expectations: Deque<&'b [u8], NUM_RESPONSE>,
}

#[derive(Format)]
pub enum Error {
    TooLittleBytes,
    TooManyBytes,
    UnexpectedCommand(&'static str),
    UnexpectedData,
}

impl<'a, 'b, I: Instance> I2cSlaveTester<'a, 'b, I> {
    pub fn new(slave: &'b mut I2cSlave<'a, I>) -> Self {
        Self {
            slave,
            read_responses: Deque::new(),
            write_expectations: Deque::new(),
        }
    }

    pub fn expect_read<const N: usize>(mut self, respond_with: &'b [u8; N]) -> Self {
        assert!(N <= BUFFER_SIZE);
        unwrap!(self.read_responses.push_front(respond_with));
        self
    }

    pub fn expect_write<const N: usize>(mut self, pattern: &'b [u8; N]) -> Self {
        assert!(N <= BUFFER_SIZE);
        unwrap!(self.write_expectations.push_front(pattern));
        self
    }

    pub async fn run(mut self) -> Result<(), Error> {
        let mut buffer = [0; BUFFER_SIZE];

        loop {
            match self.slave.listen(&mut buffer).await.unwrap() {
                embassy_rp::i2c_slave::Command::GeneralCall(_) => todo!(),
                embassy_rp::i2c_slave::Command::Read => {
                    let resp = self
                        .read_responses
                        .pop_back()
                        .ok_or(Error::UnexpectedCommand("read"))?;

                    let status = self.slave.respond_to_read(resp).await.unwrap();

                    match status {
                        embassy_rp::i2c_slave::ReadStatus::Done => {}
                        embassy_rp::i2c_slave::ReadStatus::NeedMoreBytes => {
                            return Err(Error::TooLittleBytes);
                        }
                        embassy_rp::i2c_slave::ReadStatus::LeftoverBytes(_) => {
                            return Err(Error::TooManyBytes);
                        }
                    }
                }
                embassy_rp::i2c_slave::Command::WriteRead(_) => todo!(),
                embassy_rp::i2c_slave::Command::Write(n) => {
                    let written = &buffer[..n];
                    let expected = self
                        .write_expectations
                        .pop_back()
                        .ok_or(Error::UnexpectedCommand("write"))?;

                    if written != expected {
                        error!("Expected {}, Got {}", expected, written);
                        return Err(Error::UnexpectedData);
                    }
                }
            }

            if self.read_responses.is_empty() && self.write_expectations.is_empty() {
                break;
            }
        }

        Ok(())
    }
}
