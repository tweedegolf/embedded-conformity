use embedded_hal::i2c::I2c;

pub const ADDR: u8 = 0x42;
pub struct BasicTest<I2C> {
    i2c: I2C,
}

impl<I2C: I2c> BasicTest<I2C> {
    pub fn new(i2c: I2C) -> Self {
        Self { i2c }
    }

    pub fn simple_write(&mut self) -> Result<(), I2C::Error> {
        let value = [0x76];
        self.i2c.write(ADDR, &value)?;
        Ok(())
    }
}
