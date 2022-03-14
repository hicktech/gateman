use std::error::Error;

use clap::Parser;
use rppal::gpio::Level::*;
use rppal::gpio::{Gpio, Level};
use rppal::pwm::{Channel, Polarity, Pwm};

/// Example: take steps
#[derive(Parser)]
struct Opts {
    #[clap(short, long, default_value = "500")]
    frequency: f64,

    #[clap(short, long)]
    reverse: bool,

    #[clap(long, default_value = "23")]
    clock_pin: u8,

    #[clap(long, default_value = "24")]
    data_pin: u8,

    #[clap(long, default_value = "6")]
    dir_pin: u8,

    count: u64,
}

fn main() -> Result<(), Box<dyn Error>> {
    let opts: Opts = Opts::parse();

    let mut dir = Gpio::new()?.get(opts.dir_pin)?.into_output_low();
    if opts.reverse {
        dir.set_high()
    }

    let clock = Gpio::new()?.get(opts.clock_pin)?.into_input_pullup();
    let data = Gpio::new()?.get(opts.data_pin)?.into_input_pullup();

    let mut state: u16 = 0;
    let mut encoder_idx: i64 = 0;

    let pwm = Pwm::with_frequency(Channel::Pwm0, opts.frequency, 0.5, Polarity::Normal, true)?;
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

        if encoder_idx.abs() >= opts.count as i64 {
            pwm.disable()?;
            println!("Completed {} steps", encoder_idx.abs());
            break;
        }
    }

    Ok(())
}
