use clap::Parser;

use gateman::drive::Drive;
use gateman::Result;

use git_version::git_version;
const GIT_VERSION: &str = git_version!();

/// Example: take steps
#[derive(Parser)]
#[clap(name = "Gateman", version = GIT_VERSION)]
struct Opts {
    #[clap(short, long, default_value = "500")]
    frequency: f64,

    #[clap(short, long)]
    reverse: bool,

    #[clap(long, default_value = "23")]
    clock_pin: u8,

    #[clap(long, default_value = "24")]
    data_pin: u8,

    #[clap(long, default_value = "5")]
    pub en_pin: u8,

    #[clap(long, default_value = "6")]
    dir_pin: u8,

    /// the position to move to
    #[clap(long, default_value = "0")]
    at: isize,

    /// the position to move to
    pos: isize,
}

#[tokio::main]
async fn main() -> Result<()> {
    let opts: Opts = Opts::parse();

    let mut driver = Drive::new(
        opts.at,
        opts.en_pin,
        opts.dir_pin,
        opts.clock_pin,
        opts.data_pin,
    )?;

    // todo;; this is not wired on the other end yet
    ctrlc::set_handler(|| {
        println!("received Ctrl+C!");
    })
    .expect("Error setting Ctrl-C handler");

    driver.move_to(opts.pos, None).await?;
    driver.move_to(opts.pos + 10, None).await?;

    Ok(())
}
