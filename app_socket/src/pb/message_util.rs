use crate::pb::protocol::{AuthResponse, Envelope, EnvelopeType};
use prost::Message;

use crate::pb::protocol::envelope::Payload;
use bytes::Bytes;
/// 构造 Ack 封包
use chrono::Utc;

pub fn build_auth_ack(original_envelope_id: String, message_id: String, success: bool,message:String ) -> Bytes {
    let ack_msg = AuthResponse {
        message_id: prost::alloc::string::String::from(message_id),
        success,
        message:prost::alloc::string::String::from(message),
    };

    let envelope = Envelope {
        envelope_id: prost::alloc::string::String::from(original_envelope_id),
        r#type: EnvelopeType::Ack as i32,
        timestamp: Utc::now().timestamp_millis(),
        payload: Some(Payload::AuthResponse(ack_msg)),
    };

    let mut buf = Vec::new();
    envelope.encode(&mut buf).expect("Encoding failed");
    Bytes::from(buf)
}

// pub fn build_