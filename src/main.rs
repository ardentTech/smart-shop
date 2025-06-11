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
use embassy_time::Timer;
use panic_halt as _;
use pmsa003i::{Pmsa003i, Reading};
use static_cell::StaticCell;
use crate::board::Board;

type I2c1Bus = Mutex<NoopRawMutex, I2c<'static, I2C1, Async>>;

pub enum Event {
    // TODO wrap as needed: github.com/bbustin/pmsa003i/blob/master/src/error.rs
    // need to wrap pmsa003i return value bc it uses generics and embassy async tasks cannot
    AqReadErr,
    AqReadOk(Reading),
    Nop
}

const EVENT_BUS_BUFFER_SIZE: usize = 64;
static EVENT_BUS: Channel<ThreadModeRawMutex, Event, EVENT_BUS_BUFFER_SIZE> = Channel::new();

bind_interrupts!(struct Irqs {
    I2C1_IRQ => i2c::InterruptHandler<I2C1>;
    USBCTRL_IRQ => embassy_rp::usb::InterruptHandler<USB>;
});

#[embassy_executor::task]
async fn aq_sensor(
    bus: &'static I2c1Bus,
    event_bus: Sender<'static, ThreadModeRawMutex, Event, EVENT_BUS_BUFFER_SIZE>
) {
    let device = I2cDevice::new(bus);
    let mut sensor = Pmsa003i::new(device);

    match sensor.read().await {
        Ok(reading) => event_bus.send(Event::AqReadOk(reading)).await,
        Err(_) => event_bus.send(Event::AqReadErr).await,
    }
}

#[embassy_executor::task]
async fn logging(driver: Driver<'static, USB>) {
    embassy_usb_logger::run!(1024, log::LevelFilter::Info, driver);
}

#[embassy_executor::task]
async fn event_handler() {
    loop {
        match EVENT_BUS.receive().await {
            Event::AqReadOk(data) => {
                log::info!("aq ok: {:?}", data);
                // TODO LoRa tx
            }
            Event::AqReadErr => {
                log::error!("aq err");
            }
            Event::Nop => {}
        }
    }
}

#[embassy_executor::task]
async fn orchestration(spawner: Spawner) {
    let board = Board::default();
    let usb_driver = Driver::new(board.usb, Irqs);
    spawner.must_spawn(logging(usb_driver));

    // TODO create abstraction
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
        spawner.must_spawn(aq_sensor(i2c_bus, EVENT_BUS.sender()));
        Timer::after_secs(3).await;
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    spawner.must_spawn(event_handler());
    spawner.must_spawn(orchestration(spawner));
}