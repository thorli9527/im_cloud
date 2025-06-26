#[derive(Debug)]
pub enum SendError {
    ConnectionNotFound,
    EncodeFailed(prost::EncodeError),
    ChannelClosed,
}
