use crate::entitys::mq_message_info::UserMessage;
use common::repository_util::BaseRepository;
use mongodb::Database;
use once_cell::sync::OnceCell;
use std::sync::Arc;

#[derive(Debug)]
pub struct UserMessageService {
    pub dao: BaseRepository<UserMessage>,
}

impl UserMessageService {
    pub fn new(db: Database) -> Self {
        let collection = db.collection("mq_user_message");
        Self { dao: BaseRepository::new(db, collection.clone()) }
    }
    pub fn init(db: Database) {
        let instance = Self::new(db);
        INSTANCE
            .set(Arc::new(instance))
            .expect("INSTANCE already initialized");
    }

    /// 获取单例
    pub fn get() -> Arc<Self> {
        INSTANCE
            .get()
            .expect("INSTANCE is not initialized")
            .clone()
    }
}
static INSTANCE: OnceCell<Arc<UserMessageService>> = OnceCell::new();