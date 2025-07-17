use crate::entitys::role_entity::RoleEntity;
use anyhow::Result;
use common::repository_util::{BaseRepository, Repository};
use common::util::date_util::now;
use mongodb::{bson::doc, Database};
use once_cell::sync::OnceCell;
use std::sync::Arc;
#[derive(Debug)]
pub struct RoleService {
    pub dao: BaseRepository<RoleEntity>,
}

impl RoleService {
    pub fn new(db: Database) -> Self {
        let collection = db.collection("role");
        Self { dao: BaseRepository::new(db, collection) }
    }

    /// 获取所有角色列表
    pub async fn get_all_roles(&self) -> Result<Vec<RoleEntity>> {
        let roles = self.dao.query(doc! {}).await?;
        Ok(roles)
    }

    /// 添加新角色
    pub async fn add_role(&self, name: &str, code: &str, description: Option<String>, is_builtin: bool) -> Result<String> {
        let entity = RoleEntity {
            id: "".into(),
            name: name.into(),
            code: code.into(),
            description,
            is_builtin,
            create_time: now() as u64,
            update_time: now() as u64,
        };
        self.dao.insert(&entity).await
    }

    pub fn init(db: Database) {
        INSTANCE.set(Arc::new(Self::new(db)))
            .expect("RoleService already initialized");
    }
    pub fn get() -> Arc<Self> {
        INSTANCE.get().expect("RoleService not initialized").clone()
    }
}

static INSTANCE: OnceCell<Arc<RoleService>> = OnceCell::new();
