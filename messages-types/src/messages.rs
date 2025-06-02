#[derive(Debug, Clone)]
pub enum ChatCommand {
    Subscribe(String),
    Publish(String, Vec<u8>),
    SendOne(String, Vec<u8>),
    Quit,
}
