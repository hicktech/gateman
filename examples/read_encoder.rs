use std::error::Error;

use rppal::gpio::Level::*;
use rppal::gpio::{Gpio, Level};

const CLOCK_PIN: u8 = 23;
const DATA_PIN: u8 = 24;

fn main() -> Result<(), Box<dyn Error>> {
    let clock = Gpio::new()?.get(CLOCK_PIN)?.into_input_pullup();
    let data = Gpio::new()?.get(DATA_PIN)?.into_input_pullup();

    let mut state: u16 = 0;
    let mut encoder_idx: i64 = 0;

    loop {
        let c = clock.read() as u16;
        let d = data.read() as Level;

        state = (&state << 1) | c | 0xe000;
        if state == 0xf000 {
            match d {
                High => encoder_idx += 1,
                Low => encoder_idx -= 1,
            }

            state = 0;
            println!("idx {}", encoder_idx);
        }
    }
}
