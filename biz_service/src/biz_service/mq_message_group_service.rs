use crate::biz_service::kafka_socket_service::KafkaService;
use crate::entitys::group_msg_entity::GroupMsgEntity;
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
pub struct GroupMessageService {
    pub dao: BaseRepository<GroupMsgEntity>,
}

impl GroupMessageService {
    pub async fn new(db: Database) -> Self {
        Self {
            dao: BaseRepository::new(db, "mq_group_message").await,
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
    pub async fn send_group_message(&self, from: &String, to: &String, segments: &Vec<Segment>) -> Result<GroupMsgEntity, AppError> {
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
        let message = GroupMsgEntity {
            message_id: build_snow_id(),
            from: from.to_string(),
            sync_mq_status: true,
            to: to.to_string(),
            content: segments,
            create_time: now_time,
            update_time: now_time,
            revoked: false,
            is_system: false,
            seq: 0,
        };
        let kafka_service = KafkaService::get();
        // 发送到 Kafka
        let app_config = AppConfig::get();
        let message_type = ByteMessageType::GroupMsgType;
        kafka_service.send_proto(&message_type, &message, &message.message_id, &app_config.get_kafka().topic_group).await?;
        // 持久化
        self.dao.insert(&message).await?;
        Ok(message)
    }
}
static INSTANCE: OnceCell<Arc<GroupMessageService>> = OnceCell::new();
