#![no_std]
#![no_main]

mod bsp;
mod board;

use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::i2c::{self, Async, I2c};
use embassy_rp::peripherals::{I2C1, USB};
use embassy_rp::usb::Driver;
use embassy_sync::mutex::Mutex;
use embassy_sync::blocking_mutex::raw::{NoopRawMutex, ThreadModeRawMutex};
use embassy_sync::channel::{Channel, Sender};
use panic_halt as _;
use pmsa003i::{Pmsa003i, Reading};
use static_cell::StaticCell;
use crate::board::Board;

type I2c1Bus = Mutex<NoopRawMutex, I2c<'static, I2C1, Async>>;

// need to wrap pmsa003i return value bc it uses generics and embassy async tasks cannot
// TODO use something like PeriOpResult enum to abstract all sensor ops in a single place (for use on a single channel)?
enum AQSensorResult {
    Ok(Reading),
    // TODO wrap as needed: github.com/bbustin/pmsa003i/blob/master/src/error.rs
    Err
}

const AQ_CHANNEL_BUFFER_SIZE: usize = 64;

static CHANNEL: Channel<ThreadModeRawMutex, AQSensorResult, AQ_CHANNEL_BUFFER_SIZE> = Channel::new();

bind_interrupts!(struct Irqs {
    I2C1_IRQ => i2c::InterruptHandler<I2C1>;
    USBCTRL_IRQ => embassy_rp::usb::InterruptHandler<USB>;
});

#[embassy_executor::task]
async fn aq_sensor(
    bus: &'static I2c1Bus,
    control: Sender<'static, ThreadModeRawMutex, AQSensorResult, AQ_CHANNEL_BUFFER_SIZE>
) {
    let device = I2cDevice::new(bus);
    let mut sensor = Pmsa003i::new(device);

    match sensor.read().await {
        Ok(reading) => control.send(AQSensorResult::Ok(reading)).await,
        Err(_) => control.send(AQSensorResult::Err).await,
    }
}

#[embassy_executor::task]
async fn logging(driver: Driver<'static, USB>) {
    embassy_usb_logger::run!(1024, log::LevelFilter::Info, driver);
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let board = Board::default();
    let usb_driver = Driver::new(board.usb, Irqs);
    spawner.must_spawn(logging(usb_driver));

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
    spawner.must_spawn(aq_sensor(i2c_bus, CHANNEL.sender()));

    loop {
        match CHANNEL.receive().await {
            AQSensorResult::Ok(data) => {
                log::info!("aq sensor reading: {:?}", data);
                // TODO LoRa tx
            }
            AQSensorResult::Err => {
                log::error!("aq sensor error");
            }
        }
    }
}