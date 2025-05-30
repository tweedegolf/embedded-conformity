use defmt::{Format, assert, error, trace, unwrap};
use embassy_rp::{
    i2c::Instance,
    i2c_slave::{self, I2cSlave},
};
use heapless::Deque;

pub const NUM_RESPONSE: usize = 8;
pub const BUFFER_SIZE: usize = 32;

pub struct I2cSlaveTester<'a, 'b, I: Instance> {
    slave: &'b mut I2cSlave<'a, I>,
    read: Deque<&'b [u8], NUM_RESPONSE>,
    write: Deque<&'b [u8], NUM_RESPONSE>,

    /// Expectations for the WriteRead call in the form of (Write, Read).
    write_read: Deque<(&'b [u8], &'b [u8]), NUM_RESPONSE>,
}

#[derive(Format)]
pub enum I2cSlaveTestError {
    ExpectationFailure(&'static str),
    InternalError(i2c_slave::Error),
}

impl From<i2c_slave::Error> for I2cSlaveTestError {
    fn from(value: i2c_slave::Error) -> Self {
        I2cSlaveTestError::InternalError(value)
    }
}

impl<'a, 'b, I: Instance> I2cSlaveTester<'a, 'b, I> {
    pub fn new(slave: &'b mut I2cSlave<'a, I>) -> Self {
        Self {
            slave,
            read: Deque::new(),
            write: Deque::new(),
            write_read: Deque::new(),
        }
    }

    pub fn expect_read<const N: usize>(mut self, respond_with: &'b [u8; N]) -> Self {
        assert!(N <= BUFFER_SIZE);
        assert!(!self.read.is_full());

        unwrap!(self.read.push_front(respond_with));
        self
    }

    pub fn expect_write<const N: usize>(mut self, pattern: &'b [u8; N]) -> Self {
        assert!(N <= BUFFER_SIZE);
        assert!(!self.write.is_full());

        unwrap!(self.write.push_front(pattern));
        self
    }

    pub fn expect_write_read<const W: usize, const R: usize>(
        mut self,
        write: &'b [u8; W],
        read: &'b [u8; R],
    ) -> Self {
        assert!(W <= BUFFER_SIZE && R <= BUFFER_SIZE);
        assert!(!self.write_read.is_full());

        unwrap!(self.write_read.push_front((write, read)));
        self
    }

    async fn on_read(&mut self) -> Result<(), I2cSlaveTestError> {
        let resp = self
            .read
            .pop_back()
            .ok_or(I2cSlaveTestError::ExpectationFailure(
                "Unexpected read command",
            ))?;

        trace!("responding to read");
        let status = self.slave.respond_to_read(resp).await?;

        match status {
            embassy_rp::i2c_slave::ReadStatus::Done => {}
            embassy_rp::i2c_slave::ReadStatus::NeedMoreBytes => {
                return Err(I2cSlaveTestError::ExpectationFailure(
                    "NeedMoreBytes: provided buffer exceeded",
                ));
            }
            embassy_rp::i2c_slave::ReadStatus::LeftoverBytes(_) => {
                return Err(I2cSlaveTestError::ExpectationFailure(
                    "LeftoverBytes: provided buffer too large",
                ));
            }
        }

        Ok(())
    }

    fn on_write(&mut self, written: &[u8]) -> Result<(), I2cSlaveTestError> {
        let expected = self
            .write
            .pop_back()
            .ok_or(I2cSlaveTestError::ExpectationFailure(
                "Unexpected write command",
            ))?;

        if written != expected {
            error!("Expected {}, Got {}", expected, written);
            return Err(I2cSlaveTestError::ExpectationFailure(
                "Data Received did not match expectations",
            ));
        }
        Ok(())
    }

    pub async fn run(mut self) -> Result<(), I2cSlaveTestError> {
        let mut buffer = [0; BUFFER_SIZE];
        while !self.read.is_empty() || !self.write.is_empty() || !self.write_read.is_empty() {
            match self.slave.listen(&mut buffer).await? {
                embassy_rp::i2c_slave::Command::GeneralCall(_) => defmt::unimplemented!(),
                embassy_rp::i2c_slave::Command::Read => self.on_read().await?,
                embassy_rp::i2c_slave::Command::WriteRead(n) => {
                    self.on_write(&buffer[..n])?;
                    self.on_read().await?;
                }
                embassy_rp::i2c_slave::Command::Write(n) => self.on_write(&buffer[..n])?,
            }
        }

        Ok(())
    }
}

