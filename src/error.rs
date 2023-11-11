use core::result;

use thiserror::Error;

pub type Result<T> = result::Result<T, Error>;

/// An Error that can occur in this crate
#[derive(Error, Debug)]
pub enum Error {
    #[error("{0}")]
    GpioError(#[from] rppal::gpio::Error),

    #[error("{0}")]
    PwmError(#[from] rppal::pwm::Error),

    #[error("Failed to send encoder reading")]
    EncoderTxError,

    #[error("Encoder thread error: {0}")]
    EncoderThreadError(String),

    #[error("Driver thread error: {0}")]
    DriverThreadError(String),

    #[error("Zeroing limit switch fault")]
    ZeroLimitFault,
}
