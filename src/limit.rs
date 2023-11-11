use crate::Error;
use rppal::gpio::{Gpio, InputPin};
use std::sync::Arc;

#[derive(Eq, PartialEq)]
pub enum State {
    Zero,
    Nonzero,
}

#[derive(Clone)]
pub struct LimitSwitch {
    zero: Arc<InputPin>,
    nonzero: Arc<InputPin>,
}

impl LimitSwitch {
    pub fn new(zero_pin: u8, nonzero_pin: u8) -> Result<Self, Error> {
        Ok(Self {
            zero: Arc::new(Gpio::new()?.get(zero_pin)?.into_input_pulldown()),
            nonzero: Arc::new(Gpio::new()?.get(nonzero_pin)?.into_input_pulldown()),
        })
    }

    pub fn is_zero(&self) -> Result<bool, Error> {
        Ok(self.state()? == State::Zero)
    }

    pub fn state(&self) -> Result<State, Error> {
        if self.zero.is_high() {
            Ok(State::Zero)
        } else if self.nonzero.is_high() {
            Ok(State::Nonzero)
        } else {
            Err(Error::ZeroLimitFault)
        }
    }
}
