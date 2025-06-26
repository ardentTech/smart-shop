use embassy_rp::gpio::{Input, Level, Output, Pull};
use embassy_rp::peripherals;

// TODO it's weird to reference pin #s twice...

pub struct DMA {
    pub ch0: peripherals::DMA_CH0,
    pub ch1: peripherals::DMA_CH1,
}

pub struct GPIO {
    pub p5: peripherals::PIN_5,
    pub p9: peripherals::PIN_9
}

pub struct I2C {
    pub bus: peripherals::I2C1,
    pub scl: peripherals::PIN_3,
    pub sda: peripherals::PIN_2,
}

pub struct LoRa<'a> {
    pub dio0: Input<'a>,
    pub nss: Output<'a>,
    pub reset: Output<'a>,
}

pub struct SPI {
    pub bus: peripherals::SPI1,
    pub sck: peripherals::PIN_14,
    pub mosi: peripherals::PIN_15,
    pub miso: peripherals::PIN_8,
}

pub struct Board {
    pub dma: DMA,
    pub gpio: GPIO,
    pub i2c: I2C,
    pub lora: LoRa<'static>,
    pub spi: SPI,
    pub usb: peripherals::USB
}

impl Default for Board {
    fn default() -> Self {
        let peri = embassy_rp::init(Default::default());
        Self {
            dma: DMA {
                ch0: peri.DMA_CH0,
                ch1: peri.DMA_CH1,
            },
            gpio: GPIO {
                p5: peri.PIN_5,
                p9: peri.PIN_9,
            },
            // TODO just configure I2C1 device here?
            i2c: I2C {
                bus: peri.I2C1,
                scl: peri.PIN_3,
                sda: peri.PIN_2
            },
            lora: LoRa {
                dio0: Input::new(peri.PIN_21, Pull::None),
                nss: Output::new(peri.PIN_16, Level::High),
                reset: Output::new(peri.PIN_17, Level::High),
            },
            // TODO just configure SPI1 device here?
            spi: SPI {
                bus: peri.SPI1,
                sck: peri.PIN_14,
                mosi: peri.PIN_15,
                miso: peri.PIN_8,
            },
            usb: peri.USB
        }
    }
}