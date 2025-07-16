use common::util::date_util::now;
use crate::entitys::role_permission_entity::RolePermissionEntity;
use common::repository_util::{BaseRepository, Repository};
use mongodb::{bson::doc, Database};
use anyhow::Result;
use once_cell::sync::OnceCell;
use std::sync::Arc;
#[derive(Debug)]
pub struct RolePermissionService {
    dao: BaseRepository<RolePermissionEntity>,
}

impl RolePermissionService {
    pub fn new(db: Database) -> Self {
        let collection = db.collection("role_permission");
        Self { dao: BaseRepository::new(db, collection) }
    }

    /// 给角色分配权限（创建一条关联记录）
    pub async fn assign_permission(&self, role_id: &str, permission_id: &str) -> Result<String> {
        let entity = RolePermissionEntity {
            id: "".into(),
            role_id: role_id.into(),
            permission_id: permission_id.into(),
            create_time: now() as u64,
        };
        self.dao.insert(&entity).await
    }

    /// 获取某角色的所有权限ID列表
    pub async fn get_permission_ids_by_role(&self, role_id: &str) -> Result<Vec<String>> {
        let filter = doc! { "role_id": role_id };
        let records = self.dao.query(filter).await?;
        // 提取permission_id字段
        Ok(records.into_iter().map(|rp| rp.permission_id).collect())
    }

    /// 根据多个角色ID获取所有相关的权限ID（去重）
    pub async fn get_permission_ids_by_roles(&self, role_ids: &[String]) -> Result<Vec<String>> {
        let filter = doc! { "role_id": { "$in": role_ids } };
        let records = self.dao.query(filter).await?;
        let mut perm_ids: Vec<String> = records.into_iter().map(|rp| rp.permission_id).collect();
        perm_ids.sort();
        perm_ids.dedup();
        Ok(perm_ids)
    }

    pub fn init(db: Database) {
        INSTANCE.set(Arc::new(Self::new(db)))
            .expect("RolePermissionService already initialized");
    }
    pub fn get() -> Arc<Self> {
        INSTANCE.get().expect("RolePermissionService not initialized").clone()
    }
}

static INSTANCE: OnceCell<Arc<RolePermissionService>> = OnceCell::new();
