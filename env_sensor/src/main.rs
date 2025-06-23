#![no_std]
#![no_main]

mod bsp;
mod board;

use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_futures::join::join;
use embassy_rp::i2c;
use embassy_rp::i2c::{Async, I2c};
use embassy_rp::peripherals::{I2C1, SPI1, USB};
use embassy_rp::spi::Spi;
use embassy_rp::usb::Driver;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::Timer;
use packed_struct::prelude::*;
use panic_halt as _;
use pmsa003i::Pmsa003i;
use static_cell::StaticCell;
use lora::Radio;
use sht30::Sht30;
use crate::board::Board;

pub type I2c1Bus = Mutex<NoopRawMutex, I2c<'static, I2C1, Async>>;
pub type LoRaRadio = Mutex<NoopRawMutex, Radio>;
pub type Spi1Bus = Mutex<NoopRawMutex, Spi<'static, SPI1, embassy_rp::spi::Async>>;

#[derive(Debug)]
struct AirQualityReading {
    aq_pm2_5: u16,
    aq_pm10: u16,
}
impl AirQualityReading {
    fn new(aq_pm2_5: u16, aq_pm10: u16) -> Self {
        Self { aq_pm2_5, aq_pm10 }
    }
}

#[derive(Debug)]
struct TemperatureHumidityReading {
    humidity: u16,
    temperature: u16,
}

impl TemperatureHumidityReading {
    fn new(humidity: u16, temperature: u16) -> Self {
        Self { humidity, temperature }
    }
}

#[derive(PackedStruct, Debug)]
#[packed_struct(endian="lsb")]
struct ShopReading {
    #[packed_field()]
    aq_pm2_5: u16,
    #[packed_field()]
    aq_pm10: u16,
    #[packed_field()]
    humidity: u16,
    #[packed_field()]
    temperature: u16,
}
const READ_INTERVAL_SECONDS: u64 = 3;

bind_interrupts!(struct Irqs {
    I2C1_IRQ => i2c::InterruptHandler<I2C1>;
    USBCTRL_IRQ => embassy_rp::usb::InterruptHandler<USB>;
});

async fn air_quality(
    i2c_bus: &'static I2c1Bus,
) -> Result<AirQualityReading, ()> {
    let i2c_device = I2cDevice::new(i2c_bus);
    let mut sensor = Pmsa003i::new(i2c_device);

    match sensor.read().await {
        Ok(data) => {
            Ok(AirQualityReading::new(data.pm2_5, data.pm10))
        },
        Err(e) => {
            log::error!("air quality sensor read failed: {:?}", e);
            Err(())
        },
    }
}

#[embassy_executor::task]
async fn logger(driver: Driver<'static, USB>) {
    embassy_usb_logger::run!(1024, log::LevelFilter::Info, driver);
}

#[embassy_executor::task]
async fn env_sensors(
    i2c_bus: &'static I2c1Bus,
    radio: &'static LoRaRadio,
) {
    match join(
        air_quality(i2c_bus),
        temp_humidity(i2c_bus)
    ).await {
        (Ok(aq), Ok(th)) => {
            let reading = ShopReading {
                aq_pm2_5: aq.aq_pm2_5.into(),
                aq_pm10: aq.aq_pm10.into(),
                humidity: th.humidity.into(),
                temperature: th.temperature.into(),
            };
            let payload: [u8; 8] = reading.pack().unwrap();
            radio_tx(radio, &payload).await;
            log::debug!("radio tx succeeded: {:?}", payload)
        },
        // nop bc each sensor is responsible for logging its errors
        _ => {}
    }
}

async fn radio_tx(radio: &'static LoRaRadio, data: &[u8]) {
    let mut radio = radio.lock().await;
    radio.tx(data).await.unwrap();
}

async fn temp_humidity(
    i2c_bus: &'static I2c1Bus,
) -> Result<TemperatureHumidityReading, ()> {
    let i2c_device = I2cDevice::new(i2c_bus);
    let mut sensor = Sht30::new(i2c_device);

    match sensor.read().await {
        Ok(data) => {
            Ok(TemperatureHumidityReading::new(data.humidity, data.temperature))
        },
        Err(e) => {
            log::error!("temp humidity sensor read failed: {:?}", e);
            Err(())
        },
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let board = Board::default();

    let usb_driver = Driver::new(board.usb, Irqs);
    spawner.must_spawn(logger(usb_driver));

    let spi = Spi::new(
        board.spi.bus,
        board.spi.sck,
        board.spi.mosi,
        board.spi.miso,
        board.dma.ch0,
        board.dma.ch1,
        embassy_rp::spi::Config::default()
    );
    static SPI_BUS: StaticCell<Spi1Bus> = StaticCell::new();
    let spi_bus = SPI_BUS.init(Mutex::new(spi));

    static RADIO: StaticCell<LoRaRadio> = StaticCell::new();
    let radio = RADIO.init(Mutex::new(Radio::new(spi_bus, board.lora.nss, board.lora.reset, board.lora.dio0).await));

    // TODO handle this config in Board?
    // defaults to 100 kbps, which is the only speed the AQ sensor works with
    let i2c = i2c::I2c::new_async(
        board.i2c.bus,
        board.i2c.scl,
        board.i2c.sda,
        Irqs,
        i2c::Config::default()
    );
    static I2C_BUS: StaticCell<I2c1Bus> = StaticCell::new();
    let i2c_bus = I2C_BUS.init(Mutex::new(i2c));

    loop {
        spawner.must_spawn(env_sensors(i2c_bus, radio));
        Timer::after_secs(READ_INTERVAL_SECONDS).await;
    }
}