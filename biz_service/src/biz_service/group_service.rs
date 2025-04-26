use crate::entitys::group_entity::GroupInfo;
use common::repository_util::{BaseRepository, Repository};
use mongodb::Database;
use once_cell::sync::OnceCell;
use std::sync::Arc;
use mongodb::bson::doc;
use common::errors::AppError;
use common::util::common_utils::as_ref_to_string;

#[derive(Debug)]
pub struct GroupService {
    pub dao: BaseRepository<GroupInfo>,
}

impl GroupService {
    pub fn new(db: Database) -> Self {
        let collection = db.collection("group_info");
        Self { dao: BaseRepository::new(db, collection.clone()) }
    }
    
    pub async fn find_by_group_id(&self, group_id: impl AsRef<str>) -> Result<GroupInfo, AppError> {
        let result = self.dao.find_one(doc! {"group_id":as_ref_to_string(group_id)}).await?;
        match result {
            Some(group) => Ok(group),
            None => Err(AppError::BizError("Group not found".to_string())),
        }
    }

    /// 转让群组（变更 creator_id）
    pub async fn transfer_ownership(
        &self,
        group_id: impl AsRef<str>,
        new_owner_id: impl AsRef<str>,
    ) -> Result<(), AppError> {
        let filter = doc! {"group_id": as_ref_to_string(&group_id)};
        let update = doc! {
            "$set": {
                "creator_id": as_ref_to_string(&new_owner_id),
                "update_time": common::util::date_util::now(),
            }
        };
        self.dao.update(filter, update).await?;
        Ok(())
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
static INSTANCE: OnceCell<Arc<GroupService>> = OnceCell::new();