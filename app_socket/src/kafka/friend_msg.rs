use crate::kafka::kafka_consumer::{PendingMeta, get_pending_acks};
use crate::manager::socket_manager::SocketManager;
use anyhow::{Result, anyhow};
use biz_service::protocol::protocol::envelope::Payload::FriendEvent;
use biz_service::protocol::protocol::{Envelope, EnvelopeType, FriendEventMessage};
use bytes::Buf;
use bytes::Bytes;
use common::util::date_util::now;
use std::sync::Arc;
use log::{info, warn};
use prost::Message;
use rdkafka::message::{Message as KafkaMessageTrait, OwnedMessage};

pub async fn friend_msg_to_socket(mut body: impl Buf, msg: &OwnedMessage, socket_manager: &Arc<SocketManager>) -> Result<()> {
    let message = FriendEventMessage::decode(&mut body)?;
    let message_id = message.event_id.clone();

    get_pending_acks().insert(
        message_id.clone(),
        PendingMeta {
            topic: msg.topic().to_string(),
            partition: msg.partition(),
            offset: msg.offset(),
        },
    );

    let envelope = Envelope {
        envelope_id: message_id.clone(),
        envelope_type: EnvelopeType::ServerToClient as i32,
        timestamp: now(),
        payload: Some(FriendEvent(message.clone())),
    };

    let mut buf = Vec::with_capacity(envelope.encoded_len());
    envelope.encode(&mut buf)?;
    let bytes = Bytes::from(buf);

    match socket_manager.send_to_user(&message.from_uid, bytes, None) {
        Ok(_) => {
            info!("📨 成功推送消息给用户 [{}]", &message.from_uid);
        }
        Err(e) => {
            warn!("⚠️ 推送失败 [{}]: {:?}", &message.from_uid, e);
        }
    }

    Ok(())
}
