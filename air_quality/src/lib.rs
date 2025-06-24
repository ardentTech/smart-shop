#![no_std]

use embedded_hal_async::i2c::I2c;
use pmsa003i::Pmsa003i;

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

    pub async fn read(&mut self) -> Result<AirQualityReading, ()> {
        let mut sensor = Pmsa003i::new(&mut self.i2c);

        match sensor.read().await {
            Ok(data) => {
                Ok(AirQualityReading::new(data.pm2_5, data.pm10))
            },
            // TODO wrap errors
            _ => {
                Err(())
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn it_works() {
    }
}
