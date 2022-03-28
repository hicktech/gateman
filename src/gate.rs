use crate::api;
use crate::gate::State::*;
use std::time::Duration;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
enum State {
    Closed,
    Stopped(u8),
    Moving(u8),
}

#[derive(Clone)]
pub struct GatemanRef {
    pub sender: mpsc::Sender<api::Command>,
}

impl GatemanRef {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(10);
        let actor = Gateman::new(rx);
        tokio::spawn(execute(actor));
        GatemanRef { sender: tx }
    }
}

struct Gateman {
    cmdbus: mpsc::Receiver<api::Command>,
    state: State,
}

impl Gateman {
    pub fn new(rx: mpsc::Receiver<api::Command>) -> Self {
        Gateman {
            cmdbus: rx,
            state: Closed,
        }
    }

    pub fn handle(&mut self, cmd: api::Command) {
        match cmd {
            api::Command::Close => {
                eprintln!("{:?} => Closed", self.state);
                self.state = Closed
            }
            api::Command::Open(n) => {
                // if moving, stop
                // read current position

                eprintln!("opening to {}", n);
                self.state = Moving(n)
            }
            api::Command::Stop => {
                eprintln!("Stopping");
                // todo;; need to get the current position here
                self.state = Stopped(0)
            }
        }
    }
}

async fn execute(mut actor: Gateman) {
    loop {
        let message = tokio::time::timeout(Duration::from_secs(5), actor.cmdbus.recv()).await;
        match message {
            Ok(Some(cmd)) => actor.handle(cmd),
            Ok(None) => actor.handle(api::Command::Close),
            Err(_) => {
                eprintln!("keep-alive timeout, closing");
                actor.handle(api::Command::Close)
            }
        }
    }
}
