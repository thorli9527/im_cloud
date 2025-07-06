use crate::kafka::kafka_consumer::{get_pending_acks, PendingMeta};
use crate::manager::socket_manager::SocketManager;
use anyhow::Result;
use bytes::Buf;
use common::config::AppConfig;
use prost::Message;
use rdkafka::message::{Message as KafkaMessageTrait, OwnedMessage};
use std::sync::Arc;
use biz_service::biz_service::kafka_service::KafkaService;
use biz_service::protocol::common::ByteMessageType;
use biz_service::protocol::friend::FriendEventMsg;

pub async fn friend_msg_to_socket(mut body: impl Buf, msg: &OwnedMessage, socket_manager: &Arc<SocketManager>) -> Result<()> {
    let mut message = FriendEventMsg::decode(&mut body)?;
    let message_id = message.message_id.clone();
    message.message_id = "".to_string();
    get_pending_acks().insert(message_id.clone(), PendingMeta { topic: msg.topic().to_string(), partition: msg.partition(), offset: msg.offset() });
    let friend_event=FriendEventMsg{
        message_id: "".to_string(),
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
    kafka_service.send_proto(&ByteMessageType::FriendType, &node_index, &friend_event, &friend_event.message_id, topic).await?;
    Ok(())
}
