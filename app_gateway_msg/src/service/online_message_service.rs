use crate::entity::online_message::OnLineMessageEntity;
use crate::service::kafka_service::KafkaService;
use biz_core::protocol::common::ByteMessageType;
use biz_core::protocol::msg::auth::OnlineStatusMsg;
use common::db::Db;
use common::kafka::topic_info::ONLINE_TOPIC_INFO;
use common::repository_util::{BaseRepository, Repository};
use common::util::common_utils::build_snow_id;
use std::sync::Arc;

#[derive(Debug)]
pub struct OnLineMessageService {
    pub dao: BaseRepository<OnLineMessageEntity>,
}

impl OnLineMessageService {
    pub async fn insert(&self, entity: &mut OnLineMessageEntity) -> anyhow::Result<()> {
        entity.send_group_status = false;
        self.dao.insert(entity).await?;
        let group_service = KafkaService::get();
        let kafka_list = group_service.kafka_list.lock().await;
        for kafka_service in kafka_list.iter() {
            let msg = OnlineStatusMsg {
                message_id: build_snow_id(),
                uid: entity.uid.clone(),
                device_type: entity.device_type as i32,
                client_id: entity.client_id.clone(),
                login_time: entity.login_time,
            };
            kafka_service
                .send_proto(
                    &ByteMessageType::OfflineStatusMsgType,
                    &msg,
                    &msg.message_id,
                    &ONLINE_TOPIC_INFO.topic_name,
                )
                .await?;
            self.dao.up_property(&entity.id, "send_group_status", true).await?;
        }
        return Ok(());
    }

    pub async fn new() -> Self {
        let db = Db::get();
        Self {
            dao: BaseRepository::new(db.clone(), "online_message").await,
        }
    }

    pub async fn init() {
        let service = Self::new().await;
        INSTANCE.set(Arc::new(service)).expect("init online message service error");
    }
    pub fn get() -> Arc<Self> {
        INSTANCE.get().expect("INSTANCE is not initialized").clone()
    }
}

//单例
static INSTANCE: once_cell::sync::OnceCell<Arc<OnLineMessageService>> =
    once_cell::sync::OnceCell::new();
