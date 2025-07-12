use anyhow::{anyhow, Result};
use common::errors::AppError;
use common::repository_util::{BaseRepository, Repository};
use common::util::date_util::now;
use mongodb::bson::doc;
use mongodb::Database;
use once_cell::sync::OnceCell;
use std::sync::Arc;
use common::UserId;
use crate::protocol::rpc_group_models::{GroupMemInfo, GroupRoleType};

#[derive(Debug)]
pub struct GroupMemberService {
    pub dao: BaseRepository<GroupMemInfo>,
}

impl GroupMemberService {
    pub fn new(db: Database) -> Self {
        let collection = db.collection("group_member");
        Self { dao: BaseRepository::new(db, collection.clone()) }
    }
    pub fn init(db: Database) {
        let instance = Self::new(db);
        INSTANCE.set(Arc::new(instance)).expect("INSTANCE already initialized");
    }

    pub async fn remove(&self, group_id: &str, user_id: &UserId) -> Result<(), AppError> {
        let filter = doc! {"group_id":group_id,"user_id":user_id};
        self.dao.delete(filter).await?;
        Ok(())
    }

    pub async fn delete_by_group_id(&self, group_id: &str) -> Result<(), AppError> {
        if group_id.is_empty() {
            return Err(AppError::BizError("group_id is empty".to_string()));
        }
        let filter = doc! {"group_id":group_id};
        self.dao.delete(filter).await?;
        Ok(())
    }

    pub async fn find_owner(&self, group_id: &str) -> Result<GroupMemInfo, AppError> {
        let filter = doc! {"group_id":group_id,"role":GroupRoleType::Owner as i32};
        let result = self.dao.find_one(filter).await?;
        match result {
            Some(member) => Ok(member),
            None => Err(AppError::BizError("Owner not found".to_string())),
        }
    }

    /// 仅添加群管理员
    pub async fn add_admin(&self, group_id: &str, user_id: &str) -> Result<(), AppError> {
        let member = self
            .dao
            .find_one(doc! {
                "group_id": group_id,
                "user_id": user_id,
            })
            .await?;

        match member {
            Some(m) => {
                if m.role == GroupRoleType::Owner as i32 || m.role == GroupRoleType::Admin as i32 {
                    // 已是 Owner，直接返回 OK
                    return Ok(());
                }
            }
            None => {
                // 找不到该成员
                return Err(AppError::BizError("group.member.notfound".to_string()));
            }
        }
        let filter = doc! {
            "group_id": group_id,
            "user_id": user_id,
        };
        let update = doc! {
            "$set": {
                "role": GroupRoleType::Admin as i32,
                "update_time": now(),
            }
        };
        self.dao.update(filter, update).await?;
        Ok(())
    }

    /// 取消群管理员
    pub async fn remove_admin(&self, group_id: &str, user_id: &str) -> Result<()> {
        // 查询当前成员
        let member = self
            .dao
            .find_one(doc! {
                "group_id": group_id,
                "user_id": user_id,
            })
            .await?;

        match member {
            Some(m) => {
                let role = GroupRoleType::from_i32(m.role)
                    .ok_or_else(|| anyhow!(format!("未知角色类型: {}", m.role)))?;

                match role {
                    GroupRoleType::Owner => {
                        return Err(anyhow!("群主不能降级为普通成员"));
                    }
                    GroupRoleType::Member => {
                        return Ok(());
                    }
                    GroupRoleType::Admin => {
                        // 继续执行降级操作
                    }
                }
            }
            None => {
                // 找不到成员
                return Err(anyhow!("用户 ID 不存在"));
            }
        }

        // 更新 role => Member
        let filter = doc! {
            "group_id": group_id,
            "user_id": user_id,
        };
        let update = doc! {
            "$set": {
                "role": GroupRoleType::Member as i32,
                "update_time": now(),
            }
        };

        self.dao.update(filter, update).await?;
        Ok(())
    }

    /// 获取单例
    pub fn get() -> Arc<Self> {
        INSTANCE.get().expect("INSTANCE is not initialized").clone()
    }
}
static INSTANCE: OnceCell<Arc<GroupMemberService>> = OnceCell::new();
