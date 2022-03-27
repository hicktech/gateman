#[derive(Debug, Clone)]
pub enum State {
    Closed,
    Stopped(u8),
    Moving(u8),
}

#[derive(Debug, Clone)]
pub enum Command {
    Stop,
    Close,
    Open(u8),
}
