#![no_std]

use embedded_hal_async::i2c::I2c;
use pmsa003i::{Error, Pmsa003i};

#[derive(Debug, PartialEq)]
pub enum AirQualityError<E> {
    I2C(E),
    InvalidChecksum,
    InvalidMagic
}

impl<E> From<Error<E>> for AirQualityError<E> {
    fn from(e: Error<E>) -> Self {
        match e {
            Error::I2C(e) => AirQualityError::I2C(e),
            Error::BadChecksum => AirQualityError::InvalidChecksum,
            Error::BadMagic => AirQualityError::InvalidMagic
        }
    }
}

#[derive(Debug)]
pub struct AirQualityReading {
    pub pm2_5: u16,
    pub pm10: u16,
}

impl AirQualityReading {
    fn new(pm2_5: u16, pm10: u16) -> Self {
        Self { pm2_5, pm10 }
    }
}

pub struct AQSensor<I2C> {
    i2c: I2C,
}

impl<I2C: I2c> AQSensor<I2C> {
    pub fn new(i2c: I2C) -> Self {
        Self { i2c }
    }

    pub async fn read(&mut self) -> Result<AirQualityReading, AirQualityError<I2C::Error>> {
        let mut sensor = Pmsa003i::new(&mut self.i2c);
        let data = sensor.read().await.map_err(AirQualityError::from)?;
        Ok(AirQualityReading::new(data.pm2_5, data.pm10))
    }
}

#[cfg(test)]
mod tests {
    use crate::*;
    use embedded_hal_async::i2c::ErrorKind;
    use embedded_hal_mock::eh1::i2c::{Mock as I2cMock, Transaction as I2cTransaction};

    const ADDR: u8 = 0x12;
    const RESPONSE_LEN: usize = 32;

    #[tokio::test]
    async fn read_i2c_error() {
        let expectations = [
            I2cTransaction::read(ADDR, [0u8; RESPONSE_LEN].to_vec()).with_error(ErrorKind::Other)
        ];
        let mut i2c = I2cMock::new(&expectations);
        let mut sensor = AQSensor::new(&mut i2c);
        let err = sensor.read().await.unwrap_err();
        assert_eq!(err, AirQualityError::I2C(ErrorKind::Other));
        i2c.done();
    }

    #[tokio::test]
    async fn read_invalid_magic() {
        let mut res = get_valid_response();
        res[0] = 0x00;
        let expectations = [
            I2cTransaction::read(ADDR, res.to_vec())
        ];
        let mut i2c = I2cMock::new(&expectations);
        let mut sensor = AQSensor::new(&mut i2c);
        let err = sensor.read().await.unwrap_err();
        assert_eq!(err, AirQualityError::InvalidMagic);
        i2c.done();
    }

    #[tokio::test]
    async fn read_invalid_checksum() {
        let mut res = get_valid_response();
        res[RESPONSE_LEN - 1] = 0x00;
        let expectations = [
            I2cTransaction::read(ADDR, res.to_vec())
        ];
        let mut i2c = I2cMock::new(&expectations);
        let mut sensor = AQSensor::new(&mut i2c);
        let err = sensor.read().await.unwrap_err();
        assert_eq!(err, AirQualityError::InvalidChecksum);
        i2c.done();
    }

    #[tokio::test]
    async fn read_ok() {
        let res = get_valid_response();
        let expectations = [
            I2cTransaction::read(ADDR, res.to_vec())
        ];
        let mut i2c = I2cMock::new(&expectations);
        let mut sensor = AQSensor::new(&mut i2c);
        sensor.read().await.unwrap();
        i2c.done();
    }

    fn get_valid_response() -> [u8; RESPONSE_LEN] {
        let mut res = [0x00; RESPONSE_LEN];
        // valid start of frame
        res[0] = 0x42;
        res[1] = 0x4d;
        // valid checksum
        res[30] = 0x00;
        res[31] = 0x8f;
        res
    }
}
