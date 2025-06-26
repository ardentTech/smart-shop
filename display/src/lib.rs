#![no_std]

use display_interface_i2c::I2CInterface;
use embedded_graphics::Drawable;
use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::mono_font::MonoTextStyleBuilder;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::Point;
use embedded_graphics::text::{Baseline, Text};
use embedded_hal_async::i2c::I2c;
use oled_async::Builder;
use oled_async::displayrotation::DisplayRotation;
use oled_async::displays::sh1107::Sh1107_64_128;
use oled_async::prelude::GraphicsMode;

const PIXELS: usize = 128 * 64 / 8;

pub struct Display<I2C: I2c> {
    display: GraphicsMode<Sh1107_64_128, I2CInterface<I2C>, PIXELS>
}

impl<I2C: I2c> Display<I2C> {
    pub async fn new(i2c: I2C) -> Self {
        let di = I2CInterface::new(
            i2c,
            0x3c,
            0x40
        );
        let raw_display = Builder::new(oled_async::displays::sh1107::Sh1107_64_128 {})
            .with_rotation(DisplayRotation::Rotate90)
            .connect(di);
        let mut display: GraphicsMode<_, _, PIXELS> = raw_display.into();
        // reset is mapped appropriately by stacking the oled on top of the feather
        display.init().await.unwrap();
        display.clear();
        display.flush().await.unwrap();
        Self { display }
    }

    pub async fn clear(&mut self) {
        self.display.clear();
        self.display.flush().await.unwrap();
    }

    pub async fn draw(&mut self, msg: &str) {
        let text_style = MonoTextStyleBuilder::new()
            .font(&FONT_6X10)
            .text_color(BinaryColor::On)
            .build();

        self.display.clear();
        Text::with_baseline(msg, Point::new(0, 16), text_style, Baseline::Top)
            .draw(&mut self.display)
            .unwrap();
        self.display.flush().await.unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
    }
}
