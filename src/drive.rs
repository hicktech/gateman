use std::fmt::{Debug, Display, Formatter};
use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::Arc;

use rppal::gpio::Level::*;
use rppal::gpio::{Gpio, InputPin, Level, OutputPin};
use rppal::pwm::{Channel, Polarity, Pwm};
use tokio::select;
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedSender;

use Direction::*;
use EncoderSpin::*;

use crate::Error::{DriverThreadError, EncoderThreadError, EncoderTxError};
use crate::Result;

pub struct Drive {
    en: OutputPin,
    dir: OutputPin,
    clock: Arc<InputPin>,
    data: Arc<InputPin>,
    pwm: Arc<Pwm>,
    pos: Arc<AtomicIsize>,
}

impl Drop for Drive {
    fn drop(&mut self) {
        println!("dropping driver");
        self.pwm.disable().expect("PWM failed to disable on drop");
    }
}

impl Drive {
    pub fn new(at: isize, en: u8, dir: u8, clock: u8, data: u8) -> Result<Self> {
        let en = Gpio::new()?.get(en)?.into_output_low();
        let dir = Gpio::new()?.get(dir)?.into_output_low();
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
            en,
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
    // }s

    pub fn enable(&mut self) {
        self.en.set_low()
    }

    pub fn disable(&mut self) {
        self.en.set_high()
    }

    pub async fn move_to(
        &mut self,
        target_pos: isize,
        statbus: Option<UnboundedSender<String>>,
    ) -> Result<()> {
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
            let (enc_kill_tx, enc_kill_rx) = mpsc::channel(1);

            let h = std::thread::spawn(move || read_encoder(clock, data, enc_tx, enc_kill_rx));

            // set the inferred direction
            self.dir.write(dir.into());

            // pulse steps while reading from the encoder
            let position = self.pos.clone();
            self.pwm.enable()?;

            if let Some(tx) = statbus.as_ref() {
                tx.send("begin".to_string()).unwrap();
            }

            let statbuscopy = statbus.clone();
            tokio::spawn(async move {
                loop {
                    select! {
                        Some(e) = enc_rx.recv() => {
                            match e {
                                Ccw => current_position += 1,
                                Cw => current_position -= 1,
                            };
                            encoder_steps += 1;

                            position.store(current_position, Ordering::Relaxed);
                            println!("position: {}", current_position);

                            if let Some(tx) = statbuscopy.as_ref() {
                                match tx.send(current_position.to_string()) {
                                    Ok(_) => {},
                                    Err(_) => eprintln!("failed to stat current position")
                                }
                            }

                            if target_is_met(current_position, target_pos, dir) {
                                println!(
                                    "Executed {} of projected {} steps to move to position {}",
                                    encoder_steps, steps_needed, target_pos
                                );
                                enc_kill_tx.send(()).await.expect("Failed to send encoder kill");
                            }
                        }
                        else => {
                            println!("exiting movement loop...");
                            break;
                        }
                    }
                }
            })
            .await
            .map_err(|_| DriverThreadError("Join failed".to_string()))?;

            // stop pwm
            self.pwm.disable()?;

            if let Some(tx) = statbus.as_ref() {
                tx.send("done".to_string()).unwrap();
            }

            h.join()
                .map_err(|_| EncoderThreadError("Join failed".to_string()))??;
        }

        Ok(())
    }
}

// todo;; should encoder always read
// todo;; should encoder maintain its position
fn read_encoder(
    clock: Arc<InputPin>,
    data: Arc<InputPin>,
    tx: mpsc::Sender<EncoderSpin>,
    mut kill: mpsc::Receiver<()>,
) -> Result<()> {
    let mut state: u16 = 0;
    while kill.try_recv().is_err() {
        let c = clock.read() as u16;
        let d = data.read() as Level;

        state = (&state << 1) | c | 0xe000;
        if state == 0xf000 {
            tx.blocking_send(d.into()).map_err(|_| EncoderTxError)?;
            state = 0;
        }
    }
    println!("encoder thread existing...");

    Ok(())
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

#[derive(Copy, Clone, Debug)]
enum EncoderSpin {
    Cw,
    Ccw,
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

impl From<Level> for EncoderSpin {
    fn from(l: Level) -> Self {
        match l {
            Low => Cw,
            High => Ccw,
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
