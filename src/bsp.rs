use assign_resources::assign_resources;
use embassy_rp::peripherals;

assign_resources! {
    dma: DmaResources {
        ch0: DMA_CH0,
        ch1: DMA_CH1,
    }
    gpio: GpioResources {
        p5: PIN_5,
        p6: PIN_6,
        p9: PIN_9,
        p10: PIN_10,
        p11: PIN_11,
        p12: PIN_12,
        p24: PIN_24,
        p25: PIN_25,
        p26: PIN_26,
        p27: PIN_27,
        p28: PIN_28,
        p29: PIN_29,
    }
    i2c: I2cResources {
        bus: I2C1,
        scl: PIN_3,
        sda: PIN_2,
    }
    led: LedResources {
        led: PIN_13,
        neo_pixel: PIN_4,
    }
    lora: LoraResources {
        cs: PIN_16,
        reset: PIN_17,
        io0: PIN_21,
        io1: PIN_22,
        io2: PIN_23,
        io3: PIN_19,
        io4: PIN_20,
        io5: PIN_18,
    }
    spi: SpiResources {
        spi1: SPI1,
        spi1_sck: PIN_14,
        spi1_mosi: PIN_15,
        spi1_miso: PIN_8,
    }
    uart: UartResources {
        tx: PIN_0,
        rx: PIN_1,
    },
    usb: UsbResources {
        usb: USB,
    }
}