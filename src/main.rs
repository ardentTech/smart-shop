#![no_std]
#![no_main]

mod bsp;
mod board;
mod radio;
mod shared;

use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_futures::join::join;
use embassy_rp::i2c;
use embassy_rp::peripherals::{I2C1, USB};
use embassy_rp::spi::Spi;
use embassy_rp::usb::Driver;
use embassy_sync::mutex::Mutex;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::{Channel, Sender};
use embassy_time::{Delay, Timer};
use embedded_sht3x::{Sht3x, DEFAULT_I2C_ADDRESS};
use panic_halt as _;
use pmsa003i::{Pmsa003i, Reading};
use static_cell::StaticCell;
use crate::board::Board;
use crate::radio::Radio;
use crate::shared::{I2c1Bus, LoRaRadio, Spi1Bus};

pub enum Event {
    // TODO wrap AQ Err and Reading as needed: github.com/bbustin/pmsa003i/blob/master/src/error.rs
    // need to wrap pmsa003i return value bc it uses generics and embassy async tasks cannot
    AqReadErr,
    AqReadOk(Reading),
    Nop,
    TempHumidityReadErr,
    // TODO add data to temp humidity
    TempHumidityReadOk
}

const EVENT_BUS_BUFFER_SIZE: usize = 64;
const READ_INTERVAL_SECONDS: u64 = 3;

static EVENT_BUS: Channel<ThreadModeRawMutex, Event, EVENT_BUS_BUFFER_SIZE> = Channel::new();

bind_interrupts!(struct Irqs {
    I2C1_IRQ => i2c::InterruptHandler<I2C1>;
    USBCTRL_IRQ => embassy_rp::usb::InterruptHandler<USB>;
});

async fn air_quality(
    i2c_bus: &'static I2c1Bus,
) -> Result<(), ()> {
    let i2c_device = I2cDevice::new(i2c_bus);
    let mut sensor = Pmsa003i::new(i2c_device);

    match sensor.read().await {
        Ok(data) => {
            //event_bus.send(Event::AqReadOk(data)).await
            Ok(())
        },
        Err(e) => {
            log::error!("{:?}", e);
            //event_bus.send(Event::AqReadErr).await
            Err(())
        },
    }
}

#[embassy_executor::task]
async fn event_handler(radio: &'static LoRaRadio) {
    loop {
        match EVENT_BUS.receive().await {
            Event::AqReadOk(_data) => {
                // TODO transform data to [u8]
                let data = [0, 1, 0, 1, 0, 1];
                let mut radio = radio.lock().await;
                log::info!("aq ok: {:?}", &data);
                radio.tx(&data).await.unwrap();
                log::info!("lora tx ok")
            }
            Event::AqReadErr => {
                // TODO lora tx?
            }
            Event::Nop => {}
            Event::TempHumidityReadErr => {
                // TODO lora tx?
            }
            Event::TempHumidityReadOk => {
                let data = [1, 0, 1, 0, 1, 0];
                let mut radio = radio.lock().await;
                log::info!("temp humidity ok: {:?}", &data);
                radio.tx(&data).await.unwrap();
                log::info!("lora tx ok")
            }
        }
    }
}

#[embassy_executor::task]
async fn logger(driver: Driver<'static, USB>) {
    embassy_usb_logger::run!(1024, log::LevelFilter::Info, driver);
}

#[embassy_executor::task]
async fn sensors(
    i2c_bus: &'static I2c1Bus,
    event_bus: Sender<'static, ThreadModeRawMutex, Event, EVENT_BUS_BUFFER_SIZE>
) {
    match join(
        air_quality(i2c_bus),
        temp_humidity(i2c_bus)
    ).await {
        (Ok(_), Ok(_)) => {
            log::info!("join succeeded :)");
            // TODO send event here
        },
        _ => {
            log::info!("join failed :(");
        }
    }
}

async fn temp_humidity(
    i2c_bus: &'static I2c1Bus,
) -> Result<(), ()>{
    let i2c_device = I2cDevice::new(i2c_bus);
    let mut sensor = Sht3x::new(i2c_device, DEFAULT_I2C_ADDRESS, Delay);
    match sensor.single_measurement().await {
        Ok(_data) => {
            //event_bus.send(Event::TempHumidityReadOk).await
            Ok(())
        },
        Err(e) => {
            log::error!("{:?}", e);
            //event_bus.send(Event::TempHumidityReadErr).await
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

    spawner.must_spawn(event_handler(radio));

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
        spawner.must_spawn(sensors(i2c_bus, EVENT_BUS.sender()));
        Timer::after_secs(READ_INTERVAL_SECONDS).await;
    }
}