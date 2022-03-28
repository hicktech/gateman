use rppal::gpio::{InputPin, Level};
use tokio::sync::mpsc;

pub enum Command {
    Stop,
}

pub enum Event {
    MovePos,
    MoveNeg,
}

#[derive(Clone)]
pub struct EncoderRef {
    pub mbox: mpsc::Sender<Command>,
}

impl EncoderRef {
    pub fn from_pins(tx: mpsc::Sender<Event>, clock: InputPin, data: InputPin) -> EncoderRef {
        let (mbox, mbox_rx) = mpsc::channel(10);
        let actor = Encoder::new(mbox_rx);
        tokio::spawn(execute(actor));
        std::thread::spawn(|| read_encoder(clock, data, tx));
        EncoderRef { mbox }
    }
}

pub struct Encoder {
    rx: mpsc::Receiver<Command>,
}

impl Encoder {
    fn new(rx: mpsc::Receiver<Command>) -> Self {
        Self { rx }
    }
}

async fn execute(mut actor: Encoder) {
    let message = actor.rx.recv().await;
    match message {
        Some(Command::Stop) => println!("should stop actor"),
        _ => panic!("unsupported message"),
    }
}

impl From<Level> for Event {
    fn from(l: Level) -> Self {
        match l {
            Level::High => Event::MovePos,
            Level::Low => Event::MoveNeg,
        }
    }
}

async fn read_encoder(clock: InputPin, data: InputPin, tx: mpsc::Sender<Event>) {
    let mut state: u16 = 0;
    loop {
        let c = clock.read() as u16;
        let d = data.read() as Level;

        state = (&state << 1) | c | 0xe000;
        if state == 0xf000 {
            tx.send(d.into()).await;
            state = 0;
        }
    }
}
