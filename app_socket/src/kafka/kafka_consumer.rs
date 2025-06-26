use anyhow::{Result, anyhow};
use bytes::Bytes;
use dashmap::DashMap;
use futures::TryFutureExt;
use once_cell::sync::OnceCell;
use prost::Message;
use rdkafka::message::{Message as KafkaMessageTrait, OwnedMessage};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::manager::socket_manager::SocketManager;
use biz_service::biz_service::kafka_service::KafkaMessageType;
use biz_service::manager::user_manager_core::{UserManager, UserManagerOpt};
use biz_service::protocol::protocol::{Envelope, EnvelopeType, FriendEventMessage, FriendSourceType};
use common::config::KafkaConfig;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
use biz_service::protocol::protocol::envelope::Payload;
use biz_service::protocol::protocol::envelope::Payload::FriendEvent;
use common::util::date_util::now;

type MessageId = String;

/// Kafka 消息分发结构体
#[derive(Debug, Deserialize)]
pub struct DispatchMessage {
    pub to: String,                    // 接收者用户 ID
    pub payload: String,               // 消息体（JSON/Proto 序列化后的字符串）
    pub message_id: Option<MessageId>, // 用于客户端确认的唯一标识
}

/// Kafka 消息元数据（用于 ACK 追踪）
pub struct PendingMeta {
    pub topic: String,
    pub partition: i32,
    pub offset: i64,
}

/// 全局未确认消息映射（msg_id -> 元信息）
static PENDING_ACKS: OnceCell<Arc<DashMap<MessageId, PendingMeta>>> = OnceCell::new();

pub fn get_pending_acks() -> Arc<DashMap<MessageId, PendingMeta>> {
    PENDING_ACKS.get_or_init(|| Arc::new(DashMap::new())).clone()
}

/// 全局 Kafka 消费者单例
static CONSUMER: OnceCell<Arc<StreamConsumer>> = OnceCell::new();

/// 获取 Kafka 消费者实例
pub fn get_consumer() -> Option<Arc<StreamConsumer>> {
    CONSUMER.get().cloned()
}

/// 启动 Kafka 消费循环
pub async fn start_consumer(kafka_cfg: KafkaConfig, socket_manager: Arc<SocketManager>) -> Result<()> {
    let consumer: StreamConsumer = ClientConfig::new()
        .set("group.id", "im-dispatch-group")
        .set("bootstrap.servers", &kafka_cfg.brokers)
        .set("enable.auto.commit", "false") // 手动提交 offset
        .create()?;

    consumer.subscribe(&[&kafka_cfg.topic_single, &kafka_cfg.topic_group])?;
    log::info!("✅ Kafka 消费者已启动，订阅主题：{}, {}", kafka_cfg.topic_single, kafka_cfg.topic_group);

    let arc_consumer = Arc::new(consumer);
    if CONSUMER.set(arc_consumer.clone()).is_err() {
        log::warn!("⚠️ Kafka CONSUMER 已初始化，跳过重复设置");
    }

    loop {
        match arc_consumer.recv().await {
            Ok(msg) => {
                let owned = msg.detach();
                if let Err(e) = handle_kafka_message(owned, &socket_manager).await {
                    log::error!("❌ Kafka 消息处理失败: {:?}", e);
                }
            }
            Err(e) => {
                log::error!("❌ Kafka 消费错误: {:?}", e);
            }
        }
    }
}

pub async fn handle_kafka_message(msg: OwnedMessage, socket_manager: &Arc<SocketManager>) -> Result<()> {
    let payload = msg.payload().ok_or_else(|| anyhow!("Kafka 消息为空"))?;

    if payload.is_empty() {
        return Err(anyhow!("Kafka 消息体为空"));
    }

    let msg_type = payload[0];
    let body = &payload[1..];

    match msg_type {
        x if x == KafkaMessageType::FriendMsg as u8 => {
            let message = FriendEventMessage::decode(body)?;
            let message_id = message.event_id.clone();
            get_pending_acks().insert(message_id.clone(), PendingMeta { topic: msg.topic().to_string(), partition: msg.partition(), offset: msg.offset() });
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
                Ok(_) => log::info!("📨 成功推送消息给用户 [{}]", &message.from_uid),
                Err(e) => log::warn!("⚠️ 推送失败 [{}]: {:?}", &message.from_uid, e),
            }
        }
        _ => {
            log::warn!("收到未知类型 Kafka 消息: type={}", msg_type);
        }
    }

    Ok(())
}
