use crate::entity::online_message::OnLineMessageEntity;
use common::db::Db;
use common::repository_util::BaseRepository;
use std::sync::Arc;

#[derive(Debug)]
pub struct OnLineMessageService {
    pub dao: BaseRepository<OnLineMessageEntity>,
}

impl OnLineMessageService {
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
static INSTANCE: once_cell::sync::OnceCell<Arc<OnLineMessageService>> = once_cell::sync::OnceCell::new();
