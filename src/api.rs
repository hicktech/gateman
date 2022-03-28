#[derive(Debug, Clone)]
pub enum Command {
    Stop,
    Close,
    Open(u8),
}
