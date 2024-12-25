use std::time::Duration;

use crate::{
    button::*,
    text_drawer::{DisplayError, TextDrawer, TextError},
};

use embedded_graphics::prelude::Point;
use esp_idf_hal::{
    delay::{Delay, FreeRtos},
    gpio::*,
};
use esp_idf_svc::nvs::*;
use esp_idf_sys::EspError;

use loadcell::{hx711::HX711, LoadCell};
use ssd1306::{prelude::WriteOnlyDataCommand, size::DisplaySize};

const STORAGE_NAMESPACE: &str = "scale_storage";
const SCALE_FACTOR_KEY: &str = "scale_factor";

const SCALE_TARE_NUM_SAMPLES: usize = 16;
const SCALE_CALIBRATION_NUM_SAMPLES: usize = 16;
const SCALE_CALIBRATION_WEIGHT_GRAMS: f32 = 2000.0;
const SCALE_CALIBRATION_DELAY_MS: Duration = Duration::from_millis(5);
const SCALE_SCALIBRATION_SLEEP_MS: Duration = Duration::from_millis(10);

pub enum ScaleAction {
    Tare,
    Calibrate,
}

pub struct Scale<'a, T: OutputPin, S: InputPin> {
    hx711: HX711<PinDriver<'a, T, Output>, PinDriver<'a, S, Input>, Delay>,
    button_event_handle: ButtonEventHandle,
    scale_factor: Option<f32>,
    nvs_partition: EspNvs<NvsDefault>,
    last_button_event: Option<ButtonEvent>,
}

impl<'a, T: OutputPin, S: InputPin> Scale<'a, T, S> {
    pub fn new<R: InputPin + OutputPin>(
        hx711_sck: PinDriver<'static, T, Output>,
        hx711_dt: PinDriver<'static, S, Input>,
        button: PinDriver<'static, R, Input>,
    ) -> Result<Self, EspError> {
        let mut hx711 = HX711::new(hx711_sck, hx711_dt, Delay::default());
        let button_event_handle = start_button_task(button, true).unwrap();
        hx711.set_scale(1.0);

        // Create the NVS partition
        let nvs_default_partition: EspNvsPartition<NvsDefault> = EspDefaultNvsPartition::take()?;
        let nvs = EspNvs::new(nvs_default_partition, STORAGE_NAMESPACE, true)?;

        // Try to load the scale factor from the NVS partition
        let scale_factor = nvs
            .get_u32(SCALE_FACTOR_KEY)
            .unwrap_or(None)
            .map(f32::from_bits)
            .inspect(|&scale_factor| {
                hx711.set_scale(scale_factor);
            });

        Ok(Self {
            hx711,
            button_event_handle,
            scale_factor,
            nvs_partition: nvs,
            last_button_event: None,
        })
    }

    pub fn needs_calibration(&self) -> bool {
        self.scale_factor.is_none()
    }

    pub fn tare<DI, SIZE>(
        &mut self,
        text_drawer: &mut TextDrawer<DI, SIZE>,
    ) -> Result<(), TextError<DisplayError<DI, SIZE>>>
    where
        DI: WriteOnlyDataCommand,
        SIZE: DisplaySize,
    {
        println!("Taring scale...");
        text_drawer.draw_text_clear("Taring...", Point::zero())?;
        text_drawer.flush()?;

        self.hx711.tare(SCALE_TARE_NUM_SAMPLES);
        println!("Tare complete.");
        text_drawer.draw_text_clear("Tare complete.", Point::zero())?;
        text_drawer.flush()?;

        Ok(())
    }

    fn get_avg_reading(&mut self, num_samples: usize) -> Result<f32, &'static str> {
        if num_samples == 0 {
            return Err("num_samples must be greater than 0");
        }
        let mut sum: i64 = 0;
        let mut count: usize = 0;
        loop {
            if count >= num_samples {
                break;
            }

            if let Ok(reading) = self.hx711.read() {
                sum += i64::from(reading);
                count += 1;
                FreeRtos::delay_ms(SCALE_CALIBRATION_DELAY_MS.as_millis().try_into().unwrap());
            } else {
                FreeRtos::delay_ms(SCALE_SCALIBRATION_SLEEP_MS.as_millis().try_into().unwrap());
            }
        }
        Ok((sum as f64 / count as f64) as f32)
    }

    pub fn calibrate<DI, SIZE>(
        &mut self,
        text_drawer: &mut TextDrawer<DI, SIZE>,
    ) -> Result<(), TextError<DisplayError<DI, SIZE>>>
    where
        DI: WriteOnlyDataCommand,
        SIZE: DisplaySize,
    {
        // Clear any pending button events
        self.button_event_handle.clear_events();

        println!("Starting calibration...");
        println!("Please remove any weight from the scale and press the button.");

        text_drawer.draw_text_clear_flush("Empty the scale!\nPress to continue", Point::zero())?;

        self.button_event_handle.wait_for_event(ButtonEvent::Down);

        self.tare(text_drawer)?;

        println!(
            "Please place a known weight of {} grams on the scale.",
            SCALE_CALIBRATION_WEIGHT_GRAMS
        );
        println!("Press the button when ready.");

        text_drawer.draw_text_clear_flush(
            &format!(
                "Place {}g weight\nPress to continue",
                SCALE_CALIBRATION_WEIGHT_GRAMS
            ),
            Point::zero(),
        )?;

        // Wait for the button to be pressed
        self.button_event_handle.wait_for_event(ButtonEvent::Down);

        println!(
            "Calibrating for {} samples...",
            SCALE_CALIBRATION_NUM_SAMPLES
        );

        text_drawer.draw_text_clear_flush("Calibrating...", Point::zero())?;

        let avg_result = self.get_avg_reading(SCALE_CALIBRATION_NUM_SAMPLES).unwrap();
        if avg_result == 0.0 {
            println!("Calibration failed. Average reading is 0.");
            text_drawer.draw_text_clear_flush("Calibration failed", Point::zero())?;
            return Ok(());
        }

        let scale_factor = SCALE_CALIBRATION_WEIGHT_GRAMS / avg_result;

        self.hx711.set_scale(scale_factor);
        self.scale_factor = Some(scale_factor);

        text_drawer.draw_text_clear_flush("Calibration complete", Point::zero())?;

        println!("Calibration complete. Scale factor = {}", scale_factor);
        println!("Saving calibration to NVS partition...");
        if let Some(err) = self
            .nvs_partition
            .set_u32(SCALE_FACTOR_KEY, scale_factor.to_bits())
            .err()
        {
            println!("Failed to save calibration to NVS partition: {:?}", err);
        } else {
            println!("Calibration saved to NVS partition.");
        }

        // Clear any pending button events
        self.button_event_handle.clear_events();
        Ok(())
    }

    pub fn poll_action(&mut self) -> Option<ScaleAction> {
        self.button_event_handle
            .get_event()
            .and_then(|button_event| match button_event {
                ButtonEvent::Down => {
                    self.last_button_event = Some(button_event);
                    None
                }
                ButtonEvent::Held => {
                    self.last_button_event
                        .replace(button_event)
                        .and_then(|last_event| {
                            (last_event == ButtonEvent::Down).then_some(ScaleAction::Calibrate)
                        })
                }
                ButtonEvent::Up => {
                    self.last_button_event
                        .replace(button_event)
                        .and_then(|last_event| {
                            (last_event == ButtonEvent::Down).then_some(ScaleAction::Tare)
                        })
                }
            })
    }

    pub fn poll_grams(&mut self) -> Option<f32> {
        self.hx711.read_scaled().ok()
    }
}
