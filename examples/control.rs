use std::error::Error;

use clap::Parser;
use ctrlc;
use rppal::gpio::Level::*;
use rppal::gpio::{Gpio, InputPin, Level, OutputPin};
use rppal::pwm::{Channel, Polarity, Pwm};
use std::fmt::{Debug, Display, Formatter};
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
use tokio::select;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};
use Direction::*;

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

    /// the position to move to
    #[clap(long, default_value = "0")]
    at: isize,

    /// the position to move to
    pos: isize,
}

struct Drive {
    dir: OutputPin,
    clock: Arc<InputPin>,
    data: Arc<InputPin>,
    pwm: Arc<Pwm>,
    pos: Arc<AtomicIsize>,
}

impl Drop for Drive {
    fn drop(&mut self) {
        println!("dropping driver");
        self.pwm.disable();
    }
}

impl Drive {
    fn new(at: isize, dir: u8, clock: u8, data: u8) -> Result<Self, Box<dyn Error>> {
        let mut dir = Gpio::new()?.get(dir)?.into_output_low();
        let clock = Arc::new(Gpio::new()?.get(clock)?.into_input_pullup());
        let data = Arc::new(Gpio::new()?.get(data)?.into_input_pullup());
        let pwm = Arc::new(Pwm::with_frequency(
            Channel::Pwm0,
            500f64,
            0.5,
            Polarity::Normal,
            false,
        )?);

        Ok(Self {
            dir,
            clock,
            data,
            pwm,
            pos: Arc::new(AtomicIsize::new(at)),
        })
    }

    // fn set_killer(&mut self, tx: Sender<()>, rx: Receiver<()>) {
    //     self.kill = Some((tx, rx));
    // }

    fn position(&self) -> isize {
        self.pos.load(Ordering::Relaxed)
    }

    // fn killer(self) -> Sender<()> {
    //     self.kill.0.clone()
    // }
    //
    // fn stop(self) {
    //     self.killer().send(());
    // }

    async fn move_to(&mut self, target_pos: isize) {
        let (steps_needed, dir) = steps_in_right_direction(self.position(), target_pos);
        println!("steps needed: {}", steps_needed);

        if steps_needed > 0 {
            let starting_position = self.pos.load(Ordering::Relaxed);
            let mut current_position = starting_position;
            let mut encoder_steps: usize = 0;

            println!(
                "thread: moving {} => {} ({})",
                starting_position, target_pos, dir
            );

            // start reading encoder in native thread, provie a kill channel
            let clock = self.clock.clone();
            let data = self.data.clone();
            let (enc_tx, mut enc_rx) = mpsc::channel(1);
            let (enc_kill_tx, mut enc_kill_rx) = mpsc::channel(1);

            let h = std::thread::spawn(move || read_encoder(clock, data, enc_tx, enc_kill_rx));

            // pulse steps while reading from the encoder
            self.dir.write(dir.into());
            self.pwm.enable();

            let position = self.pos.clone();

            // todo: change to select! to support the kill channel
            tokio::spawn(async move {
                loop {
                    select! {
                        Some(e) = enc_rx.recv() => {
                            match e {
                                Open => current_position += 1,
                                Close => current_position -= 1,
                            };
                            encoder_steps += 1;

                            position.store(current_position, Ordering::Relaxed);
                            println!("position: {}", current_position);

                            if target_is_met(current_position, target_pos, dir) {
                                println!(
                                    "Executed {} of projected {} steps to move to position {}",
                                    encoder_steps, steps_needed, target_pos
                                );
                                enc_kill_tx.send(()).await;
                            }
                        }
                        else => {
                            println!("exiting movement loop...");
                            break;
                        }
                    }
                }
            })
            .await;

            h.join();
            self.pwm.disable();
        }
    }
}

// todo;; should encoder always read
// todo;; should encoder maintain its position
fn read_encoder(
    clock: Arc<InputPin>,
    data: Arc<InputPin>,
    tx: mpsc::Sender<Direction>,
    mut kill: mpsc::Receiver<()>,
) {
    let mut state: u16 = 0;
    while kill.try_recv().is_err() {
        let c = clock.read() as u16;
        let d = data.read() as Level;

        state = (&state << 1) | c | 0xe000;
        if state == 0xf000 {
            tx.blocking_send(d.into());
            state = 0;
        }
    }
    println!("encoder thread existing...");
}

fn steps_in_right_direction(current: isize, target: isize) -> (isize, Direction) {
    let dir = if current < target { Open } else { Close };
    let num = (current - target).abs();
    (num, dir)
}

fn target_is_met(current: isize, target: isize, dir: Direction) -> bool {
    match dir {
        Open => current >= target,
        Close => current <= target,
    }
}

#[derive(Copy, Clone, Debug)]
enum Direction {
    Open,
    Close,
}

impl From<Direction> for Level {
    fn from(d: Direction) -> Self {
        match d {
            Open => Low,
            Close => High,
        }
    }
}

impl From<Level> for Direction {
    fn from(l: Level) -> Self {
        match l {
            Low => Open,
            High => Close,
        }
    }
}

impl Display for Direction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Open => f.write_str("Open"),
            Close => f.write_str("Close"),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let opts: Opts = Opts::parse();

    let mut driver = Drive::new(opts.at, opts.dir_pin, opts.clock_pin, opts.data_pin)?;

    // todo;; this is not wired on the other end yet
    ctrlc::set_handler(|| {
        println!("received Ctrl+C!");
    })
    .expect("Error setting Ctrl-C handler");

    driver.move_to(opts.pos).await;
    driver.move_to(opts.pos + 10).await;

    Ok(())
}
