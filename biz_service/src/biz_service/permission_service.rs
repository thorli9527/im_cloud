use common::repository_util::{BaseRepository, Repository};
use mongodb::{bson::doc, Database};
use anyhow::Result;
use once_cell::sync::OnceCell;
use std::sync::Arc;
use common::util::date_util::now;
use crate::entitys::permission_entity::PermissionEntity;
#[derive(Debug)]
pub struct PermissionService {
    pub dao: BaseRepository<PermissionEntity>,
}

impl PermissionService {
    pub fn new(db: Database) -> Self {
        let collection = db.collection("permission");
        Self { dao: BaseRepository::new(db, collection) }
    }

    /// 获取所有权限点列表
    pub async fn get_all_permissions(&self) -> Result<Vec<PermissionEntity>> {
        // 查询所有权限点（可以考虑按模块或名称排序，这里简单返回全部）
        let permissions = self.dao.query(doc! {}).await?;
        Ok(permissions)
    }

    /// 添加新权限点
    pub async fn add_permission(&self, name: &str, code: &str, module: &str,
                                description: Option<String>, enabled: bool) -> Result<String> {
        let entity = PermissionEntity {
            id: "".into(),
            name: name.into(),
            code: code.into(),
            module: module.into(),
            description,
            enabled,
            create_time: now() as u64,
            update_time: now() as u64,
        };
        // 插入新权限（code 字段在模型中标记了唯一索引）
        self.dao.insert(&entity).await
    }

    pub fn init(db: Database) {
        INSTANCE.set(Arc::new(Self::new(db)))
            .expect("PermissionService already initialized");
    }
    pub fn get() -> Arc<Self> {
        INSTANCE.get().expect("PermissionService not initialized").clone()
    }
}

static INSTANCE: OnceCell<Arc<PermissionService>> = OnceCell::new();
