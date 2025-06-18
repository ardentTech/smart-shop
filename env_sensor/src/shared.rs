use embassy_rp::i2c::{Async, I2c};
use embassy_rp::peripherals::{I2C1, SPI1};
use embassy_rp::spi::Spi;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::mutex::Mutex;
use crate::radio::Radio;

// TODO need numerical identifier for I2C and SPI?
pub type I2c1Bus = Mutex<NoopRawMutex, I2c<'static, I2C1, Async>>;
pub type LoRaRadio = Mutex<NoopRawMutex, Radio>;
pub type Spi1Bus = Mutex<NoopRawMutex, Spi<'static, SPI1, embassy_rp::spi::Async>>;