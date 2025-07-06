#[derive(Debug)]
pub enum SendError {
    ConnectionNotFound,
    EncodeError,
    ChannelClosed,
}
