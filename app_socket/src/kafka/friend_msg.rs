use crate::kafka::kafka_consumer::{get_pending_acks, PendingMeta};
use crate::socket::socket_manager::SocketManager;
use anyhow::Result;
use biz_service::kafka_util::kafka_producer::KafkaInstanceService;
use biz_service::protocol::common::ByteMessageType;
use biz_service::protocol::msg::friend::FriendEventMsg;
use bytes::Buf;
use common::config::AppConfig;
use common::util::common_utils::build_snow_id;
use prost::Message;
use rdkafka::message::{Message as KafkaMessageTrait, OwnedMessage};
use std::sync::Arc;

pub async fn friend_msg_to_socket(
    mut body: impl Buf,
    msg: &OwnedMessage,
    socket_manager: &Arc<SocketManager>,
) -> Result<()> {
    let mut message = FriendEventMsg::decode(&mut body)?;
    let message_id = message.message_id.clone();
    message.message_id = build_snow_id();
    get_pending_acks().insert(
        message_id,
        PendingMeta {
            topic: msg.topic().to_string(),
            partition: msg.partition(),
            offset: msg.offset(),
        },
    );
    let friend_event = FriendEventMsg {
        message_id: build_snow_id(),
        from_uid: "".to_string(),
        to_uid: "".to_string(),
        event_type: 0,
        message: "".to_string(),
        status: 0,
        created_at: 0,
        updated_at: 0,
        source_type: 0,
        from_a_name: "".to_string(),
        to_a_name: "".to_string(),
        from_remark: None,
        to_remark: None,
    };

    // ---------- 6. Kafka 消息通知 ----------
    let kafka_service = KafkaInstanceService::get();
    let app_config = AppConfig::get();
    let topic = &app_config.get_kafka().topic_single;
    kafka_service
        .send_proto(
            &ByteMessageType::FriendEventMsgType,
            &friend_event,
            &friend_event.message_id,
            topic,
        )
        .await?;
    Ok(())
}
