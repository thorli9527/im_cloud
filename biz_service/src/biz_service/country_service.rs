use crate::entitys::common_entity::CountryEntity;
use common::repository_util::BaseRepository;
use mongodb::Database;
use once_cell::sync::OnceCell;
use std::sync::Arc;
#[derive(Debug)]
pub struct CountryService {
    pub dao: BaseRepository<CountryEntity>,
}

impl CountryService {
    pub fn new(db: Database) -> Self {
        let collection = db.collection("country");
        Self { dao: BaseRepository::new(db, collection.clone()) }
    }
    pub fn init(db: Database) {
        let instance = Self::new(db);
        INSTANCE_COUNTRY.set(Arc::new(instance)).expect("INSTANCE already initialized");
    }

    /// 获取单例
    pub fn get() -> Arc<Self> {
        INSTANCE_COUNTRY.get().expect("INSTANCE is not initialized").clone()
    }
}
static INSTANCE_COUNTRY: OnceCell<Arc<CountryService>> = OnceCell::new();
