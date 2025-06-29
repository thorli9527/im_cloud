use crate::kafka::kafka_consumer::{get_pending_acks, PendingMeta};
use crate::manager::socket_manager::SocketManager;
use anyhow::Result;
use biz_service::biz_service::kafka_service::{ByteMessageType, KafkaService};
use biz_service::protocol::friend::FriendEventMessage;
use bytes::Buf;
use common::config::AppConfig;
use prost::Message;
use rdkafka::message::{Message as KafkaMessageTrait, OwnedMessage};
use std::sync::Arc;

pub async fn friend_msg_to_socket(mut body: impl Buf, msg: &OwnedMessage, socket_manager: &Arc<SocketManager>) -> Result<()> {
    let mut message = FriendEventMessage::decode(&mut body)?;
    let message_id = message.event_id.clone();
    message.event_id = "".to_string();
    get_pending_acks().insert(message_id.clone(), PendingMeta { topic: msg.topic().to_string(), partition: msg.partition(), offset: msg.offset() });
    let friend_event=FriendEventMessage{
        event_id: "".to_string(),
        from_uid: "".to_string(),
        to_uid: "".to_string(),
        event_type: 0,
        message: "".to_string(),
        status: 0,
        created_at: 0,
        updated_at: 0,
        source_type: 0,
    };

    // ---------- 6. Kafka 消息通知 ----------
    let kafka_service = KafkaService::get();
    let app_config = AppConfig::get();
    let node_index=0 as u8;
    let topic = &app_config.kafka.topic_single;
    kafka_service.send_proto(&ByteMessageType::FriendMsg, &node_index, &friend_event, &friend_event.event_id, topic).await?;
    Ok(())
}
