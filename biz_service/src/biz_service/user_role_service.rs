use crate::biz_service::role_service::RoleService;
use crate::entitys::role_entity::RoleEntity;
use crate::entitys::user_role_entity::UserRoleEntity;
use anyhow::Result;
use common::repository_util::{BaseRepository, Repository};
use common::util::date_util::now;
use mongodb::{Database, bson::doc};
use once_cell::sync::OnceCell;
use std::sync::Arc;

#[derive(Debug)]
pub struct UserRoleService {
    dao: BaseRepository<UserRoleEntity>,
}

impl UserRoleService {
    pub fn new(db: Database) -> Self {
        let collection = db.collection("user_role");
        Self { dao: BaseRepository::new(db, collection) }
    }

    /// 给用户分配角色
    pub async fn assign_role(&self, user_id: &str, role_id: &str) -> Result<String> {
        let entity = UserRoleEntity {
            id: "".into(),
            user_id: user_id.into(),
            role_id: role_id.into(),
            create_time: now() as u64,
        };
        self.dao.insert(&entity).await
    }

    /// 获取某用户的所有角色ID列表
    pub async fn get_role_ids_by_user(&self, user_id: &str) -> Result<Vec<String>> {
        let filter = doc! { "user_id": user_id };
        let records = self.dao.query(filter).await?;
        Ok(records.into_iter().map(|ur| ur.role_id).collect())
    }

    /// 获取某用户的所有角色实体列表（附加功能）
    pub async fn get_roles_by_user(&self, user_id: &str) -> Result<Vec<RoleEntity>> {
        let role_ids = self.get_role_ids_by_user(user_id).await?;
        if role_ids.is_empty() {
            return Ok(vec![]);
        }
        // 查询role集合获取角色详情
        let roles = RoleService::get().dao.query(doc! { "id": { "$in": &role_ids } }).await?;
        Ok(roles)
    }

    pub fn init(db: Database) {
        INSTANCE.set(Arc::new(Self::new(db))).expect("UserRoleService already initialized");
    }
    pub fn get() -> Arc<Self> {
        INSTANCE.get().expect("UserRoleService not initialized").clone()
    }
}

static INSTANCE: OnceCell<Arc<UserRoleService>> = OnceCell::new();
