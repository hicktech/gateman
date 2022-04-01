use clap::Parser;

/// Gateman
/// Websocket based service that provides remote gate control.
#[derive(Parser)]
#[clap(name = "Gateman", version = "v0.0.0")]
pub struct Opts {
    #[clap(short, long, default_value = "500")]
    pub frequency: f64,

    #[clap(long, default_value = "23")]
    pub clock_pin: u8,

    #[clap(long, default_value = "24")]
    pub data_pin: u8,

    #[clap(long, default_value = "6")]
    pub dir_pin: u8,

    /// Used to zero the encoder
    #[clap(long, default_value = "0")]
    pub at: isize,
}
