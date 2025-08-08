use crate::entitys::user_msg_entity::UserMsgEntity;
use crate::kafka_util::kafka_producer::KafkaInstanceService;
use crate::protocol::common::ByteMessageType;
use crate::protocol::msg::message::Segment;
use common::config::AppConfig;
use common::errors::AppError;
use common::repository_util::{BaseRepository, Repository};
use common::util::common_utils::{build_snow_id, build_uuid};
use common::util::date_util::now;
use mongodb::Database;
use once_cell::sync::OnceCell;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug)]
pub struct UserMessageService {
    pub dao: BaseRepository<UserMsgEntity>,
}

impl UserMessageService {
    pub async fn new(db: Database) -> Self {
        Self {
            dao: BaseRepository::new(db, "mq_user_message").await,
        }
    }
    pub async fn init(db: Database) {
        let instance = Self::new(db).await;
        INSTANCE.set(Arc::new(instance)).expect("INSTANCE already initialized");
    }

    /// 获取单例
    pub fn get() -> Arc<Self> {
        INSTANCE.get().expect("INSTANCE is not initialized").clone()
    }
    /// 构造并保存一条用户消息，返回完整 UserMessage
    pub async fn send_user_message(
        &self,
        from: &String,
        to: &String,
        segments: &Vec<Segment>,
    ) -> Result<UserMsgEntity, AppError> {
        let now_time = now();
        if segments.is_empty() {
            return Err(AppError::BizError("消息内容不能为空".into()));
        }
        // 构造 Segment 数组
        let segments: Vec<Segment> = segments
            .iter()
            .enumerate()
            .map(|(_, s)| Segment {
                body: s.body.clone(),
                segment_id: build_uuid(),
                seq_in_msg: build_snow_id() as u64, // 分布式唯一顺序段号
                edited: false,
                visible: true,
                metadata: { HashMap::new() },
            })
            .collect();

        // 构造 UserMessage 对象
        let message = UserMsgEntity {
            message_id: build_snow_id(),
            from: from.clone(),
            to: to.clone(),
            content: segments,
            created_time: now_time,
            updated_time: now_time,
            sync_mq_status: false,
            revoked: false,
            is_system: false,
            delivered: false,
            read_time: now_time,
        };
        let kafka_service = KafkaInstanceService::get();
        // 发送到 Kafka
        let app_config = AppConfig::get();
        let msg_type: ByteMessageType = ByteMessageType::UserMsgType;
        kafka_service
            .send_proto(
                &msg_type,
                &message,
                &message.message_id,
                &app_config.get_kafka().topic_single,
            )
            .await?;
        // 持久化
        self.dao.insert(&message).await?;
        Ok(message)
    }
}
static INSTANCE: OnceCell<Arc<UserMessageService>> = OnceCell::new();
