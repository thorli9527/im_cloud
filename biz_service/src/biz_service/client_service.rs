use crate::protocol::common::ClientEntity;
use common::repository_util::BaseRepository;
use mongodb::Database;
use once_cell::sync::OnceCell;
use std::sync::Arc;

#[derive(Debug)]
pub struct ClientService {
    pub dao: BaseRepository<ClientEntity>,
}

impl ClientService {
    pub async fn new(db: Database) -> Self {
        let collection = db.collection("client");
        let result = Self {
            dao: BaseRepository::new(db, collection.clone(), "client").await,
        };
        return result;
    }

    pub async fn init(db: Database) {
        let instance = Self::new(db).await;
        INSTANCE.set(Arc::new(instance)).expect("INSTANCE already initialized");
    }

    /// 获取单例
    pub fn get() -> Arc<Self> {
        INSTANCE.get().expect("INSTANCE is not initialized").clone()
    }
}
static INSTANCE: OnceCell<Arc<ClientService>> = OnceCell::new();
