use embassy_rp::peripherals;

pub struct I2C {
    pub bus: peripherals::I2C1,
    pub scl: peripherals::PIN_3,
    pub sda: peripherals::PIN_2,
}

pub struct Board {
    pub i2c: I2C,
    pub usb: peripherals::USB
}

impl Default for Board {
    fn default() -> Self {
        let peri = embassy_rp::init(Default::default());
        Self {
            i2c: I2C {
                bus: peri.I2C1,
                scl: peri.PIN_3,
                sda: peri.PIN_2
            },
            usb: peri.USB
        }
    }
}