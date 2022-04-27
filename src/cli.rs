use std::str::FromStr;

use clap::Parser;
use git_version::git_version;

const GIT_VERSION: &str = git_version!();

/// Websocket controller for gate.
#[derive(Parser)]
#[clap(name = "Gateman", version = GIT_VERSION)]
pub struct Opts {
    #[clap(long, default_value = "127.0.0.1")]
    pub address: NetInterface,

    #[clap(long, default_value = "9000")]
    pub port: u16,

    #[clap(short, long, default_value = "500")]
    pub frequency: f64,

    #[clap(long, default_value = "23")]
    pub clock_pin: u8,

    #[clap(long, default_value = "24")]
    pub data_pin: u8,

    #[clap(long, default_value = "5")]
    pub en_pin: u8,

    #[clap(long, default_value = "6")]
    pub dir_pin: u8,

    /// Used to zero the encoder
    #[clap(long, default_value = "0")]
    pub at: isize,
}

#[derive(Debug)]
pub enum NetInterface {
    Loopback,
    OOOO,
}

impl FromStr for NetInterface {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "127.0.0.1" => Ok(NetInterface::Loopback),
            "0.0.0.0" => Ok(NetInterface::OOOO),
            unsupported => Err(format!("{} is not a valid interface", unsupported)),
        }
    }
}

impl From<NetInterface> for [u8; 4] {
    fn from(i: NetInterface) -> Self {
        match i {
            NetInterface::Loopback => [127, 0, 0, 1],
            NetInterface::OOOO => [0, 0, 0, 0],
        }
    }
}
