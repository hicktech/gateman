use std::time::Duration;

use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedSender;

use crate::drive::Drive;
use crate::gate::State::*;
use crate::Result;

#[derive(Debug, Clone)]
pub enum Command {
    Close,
    Open(u8),
    Connect(UnboundedSender<String>),
    Nop,
}

#[derive(Debug, Clone)]
enum State {
    Stopped(u8),
    Moving(u8),
}

#[derive(Clone)]
pub struct GatemanRef {
    pub sender: mpsc::Sender<Command>,
}

impl GatemanRef {
    pub fn new(driver: Drive) -> Self {
        let (tx, rx) = mpsc::channel(10);
        let actor = Gateman::new(driver, rx);
        tokio::spawn(execute(actor));
        GatemanRef { sender: tx }
    }
}

struct Gateman {
    driver: Drive,
    cmdbus: mpsc::Receiver<Command>,
    statbus: Option<UnboundedSender<String>>,
    state: State,
}

impl Gateman {
    pub fn new(driver: Drive, rx: mpsc::Receiver<Command>) -> Self {
        Gateman {
            driver,
            cmdbus: rx,
            statbus: None,
            state: Stopped(0),
        }
    }

    pub async fn handle(&mut self, cmd: Command) -> Result<()> {
        match cmd {
            Command::Nop => {}
            Command::Close => {
                eprintln!("{:?} => Closed", self.state);
                self.state = Moving(0);
                self.driver.enable();
                // todo;; error here does not disable stepper
                self.driver.move_to(0, self.statbus.clone()).await?;
                self.driver.disable();
                self.state = Stopped(0)
            }
            Command::Open(n) => {
                // todo;; if moving, stop?
                eprintln!("opening to {}", n);
                self.state = Moving(n);
                self.driver.enable();
                // todo;; error here does not disable stepper
                // todo;; externalize this multiplier
                self.driver
                    .move_to(n as isize * 35, self.statbus.clone())
                    .await?;
                self.driver.disable();
                eprintln!("completed move to {}", n);
            }
            Command::Connect(tx) => self.statbus = Some(tx),
        }
        Ok(())
    }
}

async fn execute(mut actor: Gateman) -> Result<()> {
    loop {
        let message = tokio::time::timeout(Duration::from_secs(5), actor.cmdbus.recv()).await;
        match message {
            Ok(Some(cmd)) => actor.handle(cmd),
            Ok(None) => actor.handle(Command::Close),
            Err(_) => {
                eprintln!("keep-alive timeout, closing");
                actor.handle(Command::Close)
            }
        }
        .await?
    }
}
