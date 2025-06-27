#![no_std]
#![no_main]

mod board;

use core::fmt::Write;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_embedded_hal::shared_bus::I2cDeviceError;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_futures::join::join;
use embassy_futures::select::{select, Either};
use embassy_rp::gpio::{Input, Pull};
use embassy_rp::i2c;
use embassy_rp::i2c::{Async, Error, I2c};
use embassy_rp::peripherals::{I2C1, SPI1, USB};
use embassy_rp::spi::Spi;
use embassy_rp::usb::Driver;
use embassy_sync::blocking_mutex::raw::{CriticalSectionRawMutex, NoopRawMutex};
use embassy_sync::channel::{Channel, Receiver, Sender};
use embassy_sync::mutex::Mutex;
use embassy_sync::signal::Signal;
use embassy_time::Timer;
use heapless::String;
use packed_struct::prelude::*;
use panic_halt as _;
use static_cell::StaticCell;
use air_quality::{AQSensor, AirQualityError, AirQualityReading};
use display::Display;
use lora_radio::LoraRadio;
use sht30::{Sht30, Sht30Error, Sht30Reading};
use crate::board::Board;

pub type I2c1Bus = Mutex<NoopRawMutex, I2c<'static, I2C1, Async>>;
pub type LoRaRadio = Mutex<NoopRawMutex, LoraRadio>;
pub type Spi1Bus = Mutex<NoopRawMutex, Spi<'static, SPI1, embassy_rp::spi::Async>>;

enum Event {
    DisplayActivated,
    DisplayDeactivated,
}
static CHANNEL: Channel<CriticalSectionRawMutex, Event, 64> = Channel::new();

static LAST_ENV_READING: Signal<CriticalSectionRawMutex, EnvReading> = Signal::new();

#[derive(PackedStruct, Clone, Debug)]
#[packed_struct(endian="lsb")]
struct EnvReading {
    #[packed_field()]
    aq_pm2_5: u16,
    #[packed_field()]
    aq_pm10: u16,
    #[packed_field()]
    humidity: u16,
    #[packed_field()]
    temperature: u16,
}

impl Into<String<64>> for EnvReading {
    fn into(self) -> String<64> {
        let mut msg: String<64> = String::new();
        core::write!(&mut msg, "Temp: {}F\nHumidity: {}%\nAQ PM 2.5: {}\nAQ PM 10: {}", self.temperature, self.humidity, self.aq_pm2_5, self.aq_pm10).unwrap();
        msg
    }
}


const READ_INTERVAL_SECONDS: u64 = 3;

bind_interrupts!(struct Irqs {
    I2C1_IRQ => i2c::InterruptHandler<I2C1>;
    USBCTRL_IRQ => embassy_rp::usb::InterruptHandler<USB>;
});

async fn air_quality(
    i2c_bus: &'static I2c1Bus,
) -> Result<AirQualityReading, AirQualityError<I2cDeviceError<Error>>> {
    let i2c_device = I2cDevice::new(i2c_bus);
    let mut sensor = AQSensor::new(i2c_device);
    sensor.read().await
}

#[embassy_executor::task]
async fn display(
    control: Receiver<'static, CriticalSectionRawMutex, Event, 64>,
    i2c_bus: &'static I2c1Bus,
) {
    let i2c_device = I2cDevice::new(i2c_bus);
    let mut oled = Display::new(i2c_device).await;

    loop {
        match control.receive().await {
            Event::DisplayActivated => {
                let reading = LAST_ENV_READING.wait().await;
                let msg: String<64> = reading.into();
                oled.draw(&*msg).await
            }
            Event::DisplayDeactivated => {
                oled.clear().await
            }
        }
    }
}

#[embassy_executor::task]
async fn display_controls(
    mut btn_a: Input<'static>,
    mut btn_c: Input<'static>,
    control: Sender<'static, CriticalSectionRawMutex, Event, 64>
) {
    loop {
        match select(btn_a.wait_for_falling_edge(), btn_c.wait_for_falling_edge()).await {
            Either::First(_) => control.send(Event::DisplayActivated).await,
            Either::Second(_) => control.send(Event::DisplayDeactivated).await
        }
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
            let reading = EnvReading {
                aq_pm2_5: aq.pm2_5.into(),
                aq_pm10: aq.pm10.into(),
                humidity: th.humidity.into(),
                temperature: th.temperature.into(),
            };
            LAST_ENV_READING.signal(reading.clone());

            let payload: [u8; 8] = reading.pack().unwrap();
            radio_tx(radio, &payload).await;
            log::debug!("radio tx succeeded: {:?}", payload)
        },
        // nop bc each sensor is responsible for logging its errors
        _ => {}
    }
}

// TODO should this return a result as well?
async fn radio_tx(radio: &'static LoRaRadio, data: &[u8]) {
    let mut radio = radio.lock().await;
    radio.tx(data).await.unwrap();
}

async fn temp_humidity(
    i2c_bus: &'static I2c1Bus,
) -> Result<Sht30Reading, Sht30Error<I2cDeviceError<Error>>> {
    let i2c_device = I2cDevice::new(i2c_bus);
    let mut sensor = Sht30::new(i2c_device);
    sensor.read().await
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
    let radio = RADIO.init(Mutex::new(LoraRadio::new(spi_bus, board.lora.nss, board.lora.reset, board.lora.dio0).await));

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

    let btn_a = Input::new(board.gpio.p9, Pull::Up);
    let btn_c = Input::new(board.gpio.p5, Pull::Up);
    spawner.must_spawn(display_controls(btn_a, btn_c, CHANNEL.sender()));
    spawner.must_spawn(display(CHANNEL.receiver(), i2c_bus));

    loop {
        spawner.must_spawn(env_sensors(i2c_bus, radio));
        Timer::after_secs(READ_INTERVAL_SECONDS).await;
    }
}