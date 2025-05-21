use defmt::unwrap;
use embassy_rp::{i2c::Instance, i2c_slave::I2cSlave};
use heapless::{Deque, Vec};

pub const NUM_RESPONSE: usize = 8;
pub const BUFFER_SIZE: usize = 32;

pub struct I2cSlaveTester<'a, 'b, I: Instance> {
    slave: &'b mut I2cSlave<'a, I>,
    read_responses: heapless::Deque<&'b [u8], NUM_RESPONSE>,
}

impl<'a, 'b, I: Instance> I2cSlaveTester<'a, 'b, I> {
    pub fn new(slave: &'b mut I2cSlave<'a, I>) -> Self {
        Self {
            slave,
            read_responses: Deque::new(),
        }
    }

    pub fn expect_read<const N: usize>(mut self, respond_with: &'b [u8; N]) -> Self {
        unwrap!(self.read_responses.push_front(respond_with));
        self
    }

    pub async fn run(mut self) -> Result<(), ()> {
        // TODO: Better errors than just bool

        let mut buffer = [0; BUFFER_SIZE];
        loop {
            match self.slave.listen(&mut buffer).await.unwrap() {
                embassy_rp::i2c_slave::Command::GeneralCall(_) => todo!(),
                embassy_rp::i2c_slave::Command::Read => {
                    let resp = self.read_responses.pop_back().ok_or(())?;

                    let status = self.slave.respond_to_read(resp).await.unwrap();

                    match status {
                        embassy_rp::i2c_slave::ReadStatus::Done => {}
                        embassy_rp::i2c_slave::ReadStatus::NeedMoreBytes => return Err(()),
                        embassy_rp::i2c_slave::ReadStatus::LeftoverBytes(_) => return Err(()),
                    }
                }
                embassy_rp::i2c_slave::Command::WriteRead(_) => todo!(),
                embassy_rp::i2c_slave::Command::Write(_) => todo!(),
            }

            if self.read_responses.is_empty() {
                break;
            }
        }

        Ok(())
    }
}
