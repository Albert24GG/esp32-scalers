mod button;
mod scale;
mod text_drawer;

use embedded_graphics::{mono_font::ascii::FONT_7X13_BOLD, prelude::*};
use esp_idf_hal::{
    delay::FreeRtos,
    gpio::*,
    i2c::{I2cConfig, I2cDriver},
    peripherals::Peripherals,
    prelude::*,
};
use scale::*;
use text_drawer::*;

use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306};

fn main() -> anyhow::Result<()> {
    esp_idf_hal::sys::link_patches();

    let peripherals = Peripherals::take()?;

    // Create the display
    let mut display = {
        let i2c = peripherals.i2c0;
        let sda = peripherals.pins.gpio21;
        let scl = peripherals.pins.gpio22;
        let config = I2cConfig::new().baudrate(400.kHz().into());
        let i2c_driver = I2cDriver::new(i2c, sda, scl, &config)?;
        let i2c_interface = I2CDisplayInterface::new(i2c_driver);
        Ssd1306::new(i2c_interface, DisplaySize128x32, DisplayRotation::Rotate0)
            .into_buffered_graphics_mode()
    };

    // Initialize the display
    display.init().unwrap();

    // Create the text drawer
    let mut text_drawer = TextDrawer::new(display, &FONT_7X13_BOLD);

    // Create the scale
    let mut scale = {
        let hx711_dt = PinDriver::input(peripherals.pins.gpio16)?;
        let hx711_sck = PinDriver::output(peripherals.pins.gpio4)?;
        let button = PinDriver::input(peripherals.pins.gpio17)?;
        Scale::new(hx711_sck, hx711_dt, button)?
    };

    scale.tare(&mut text_drawer)?;
    if scale.needs_calibration() {
        scale.calibrate(&mut text_drawer)?;
    }

    loop {
        let scale_action = scale.poll_action();

        if let Some(action) = scale_action {
            match action {
                ScaleAction::Tare => {
                    scale.tare(&mut text_drawer)?;
                }
                ScaleAction::Calibrate => {
                    scale.calibrate(&mut text_drawer)?;
                }
            }
        }

        if let Some(grams) = scale.poll_grams() {
            println!("Weight: {}g", grams);
            let fmt_string = if grams.abs() > 1000.0 {
                format!("Weight: {:.2}kg", grams / 1000.0)
            } else {
                format!("Weight {}g", grams.round_ties_even() as i32)
            };
            text_drawer.draw_text_clear_flush(&fmt_string, Point::zero())?;
        }

        FreeRtos::delay_ms(500u32);
    }
}
