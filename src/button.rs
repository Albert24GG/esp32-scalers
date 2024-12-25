use std::time::{Duration, Instant};

use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::{Input, InputPin, Level, OutputPin, PinDriver, Pull};
use esp_idf_sys::EspError;
use log::{error, info};
use std::sync::mpsc::{channel, Receiver, Sender};

const CONFIG_ESP32_BUTTON_LONG_PRESS_DURATION_MS: Duration = Duration::from_millis(3000);

const CONFIG_ESP32_POLLING_PERIOD_MS: Duration = Duration::from_millis(10);

const HISTORY_MASK: u16 = 0b1111_0000_0011_1111;

#[derive(PartialEq, Eq)]
pub enum ButtonEvent {
    Up,
    Down,
    Held,
}

#[derive(Default)]
struct Button {
    inverted: bool,
    history: u16,
    down_time: Option<Instant>,
    next_long_time: Option<Instant>,
}

pub struct ButtonEventHandle {
    event_queue: Receiver<ButtonEvent>,
}

impl Button {
    pub fn new(inverted: bool) -> Self {
        Self {
            inverted,
            history: if inverted { 0xFFFF } else { 0x0000 },
            ..Default::default()
        }
    }

    fn start_task<T: InputPin + OutputPin>(
        mut self,
        pin: PinDriver<'static, T, Input>,
        event_sender: Sender<ButtonEvent>,
    ) {
        std::thread::spawn(move || loop {
            self.button_update(&pin);

            if self.down_time.is_some() && self.button_up() {
                self.down_time = None;
                info!("Button Up");
                event_sender.send(ButtonEvent::Up).unwrap();
            } else if let (Some(_down_time), Some(next_long_time)) =
                (self.down_time, self.next_long_time)
            {
                if Instant::now() >= next_long_time {
                    info!("Button Held");
                    self.next_long_time = None;
                    event_sender.send(ButtonEvent::Held).unwrap();
                }
            } else if self.down_time.is_none() && self.button_down() {
                self.down_time = Some(Instant::now());
                self.next_long_time =
                    Some(self.down_time.unwrap() + CONFIG_ESP32_BUTTON_LONG_PRESS_DURATION_MS);
                info!("Button Down");
                event_sender.send(ButtonEvent::Down).unwrap();
            }

            FreeRtos::delay_ms(
                CONFIG_ESP32_POLLING_PERIOD_MS
                    .as_millis()
                    .try_into()
                    .unwrap(),
            );
        });
    }

    fn button_rose(&mut self) -> bool {
        if self.history & HISTORY_MASK == 0b0000_0000_0011_1111 {
            self.history = 0xFFFF;
            true
        } else {
            false
        }
    }

    fn button_fell(&mut self) -> bool {
        if self.history & HISTORY_MASK == 0b1111_0000_0000_0000 {
            self.history = 0x0000;
            true
        } else {
            false
        }
    }

    fn button_up(&mut self) -> bool {
        if self.inverted {
            self.button_rose()
        } else {
            self.button_fell()
        }
    }

    fn button_down(&mut self) -> bool {
        if self.inverted {
            self.button_fell()
        } else {
            self.button_rose()
        }
    }

    fn button_update<T: InputPin>(&mut self, button_pin: &PinDriver<T, Input>) {
        let level_value: u16 = (button_pin.get_level() == Level::High).into();
        self.history = (self.history << 1) | level_value;
    }
}

impl ButtonEventHandle {
    pub fn get_event(&self) -> Option<ButtonEvent> {
        self.event_queue.try_recv().ok()
    }

    /// Wait for a specific event to occur
    pub fn wait_for_event(&self, event: ButtonEvent) {
        loop {
            match self.event_queue.recv() {
                Ok(received_event) => {
                    if received_event == event {
                        break;
                    }
                }
                Err(_) => {
                    error!("Error receiving event");
                    break;
                }
            }
        }
    }

    pub fn clear_events(&self) {
        while self.event_queue.try_recv().is_ok() {}
    }
}

pub fn start_button_task<T: InputPin + OutputPin>(
    mut pin: PinDriver<'static, T, Input>,
    inverted: bool,
) -> Result<ButtonEventHandle, EspError> {
    let (tx, rx) = channel();

    let button = Button::new(inverted);

    pin.set_pull(if inverted { Pull::Up } else { Pull::Down })?;

    button.start_task(pin, tx);

    Ok(ButtonEventHandle { event_queue: rx })
}
