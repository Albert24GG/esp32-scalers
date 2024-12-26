# Simple weighing scale with HX711 and ESP32

## Demo

The project can be tested using wokwi [here](https://wokwi.com/projects/418191710587949057).

## Prerequisites

Follow [this guide](https://github.com/esp-rs/esp-idf-template#prerequisites) and make sure you have all the prerequisites installed.

## Setup

1. Clone this repository
2. Build the project using cargo

```bash
$ cargo build --release
```

3. Connect your ESP32 and flash the firmware

```bash
$ cargo espflash --release
```

4. Optionally, open a serial monitor to see the output and all logs

```bash
$ cargo espflash monitor
```

## Usage

For the first usage, you need to calibrate the scale. To do this, follow these steps (also shown on the screen and in the serial monitor):

1. Remove all weight from the scale
2. Press the button
3. Wait for the taring process to finish, then put a known weight on the scale
4. Press the button again
5. Wait for the calibration process to finish

After the calibration process, you can use the scale. Just put the weight on the scale and the weight will be shown on the screen.
The button has 2 functions:

- Short press: tare the scale
- Long press: recalibrate the scale

## Wiring

| HX711 | ESP32 |
| ----- | ----- |
| DT    | 16    |
| SCK   | 4     |

| Button | ESP32 |
| ------ | ----- |
| 1      | 17    |
| 2      | GND   |

| Display | ESP32 |
| ------- | ----- |
| SDA     | 21    |
| SCL     | 22    |
| VCC     | 3.3V  |
| GND     | GND   |
