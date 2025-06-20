use crate::entitys::group_member::{GroupMember, GroupRole};
use crate::manager::common::UserId;
use anyhow::Result;
use common::errors::AppError;
use common::repository_util::{BaseRepository, Repository};
use common::util::common_utils::as_ref_to_string;
use common::util::date_util::now;
use mongodb::bson::doc;
use mongodb::Database;
use once_cell::sync::OnceCell;
use std::sync::Arc;

#[derive(Debug)]
pub struct GroupMemberService {
    pub dao: BaseRepository<GroupMember>,
}

impl GroupMemberService {
    pub fn new(db: Database) -> Self {
        let collection = db.collection("group_member");
        Self { dao: BaseRepository::new(db, collection.clone()) }
    }
    pub fn init(db: Database) {
        let instance = Self::new(db);
        INSTANCE
            .set(Arc::new(instance))
            .expect("INSTANCE already initialized");
    }
    
    pub async fn remove(&self, group_id: impl AsRef<str>, user_id: &UserId) -> Result<(), AppError> {
        let filter = doc! {"group_id":as_ref_to_string(group_id),"user_id":user_id.to_string()};
        self.dao.delete(filter).await?;
        Ok(())
    }
    
    pub async fn delete_by_group_id(&self, group_id: impl AsRef<str>) -> Result<(), AppError> {
        if as_ref_to_string(&group_id).is_empty(){
            return Err(AppError::BizError("group_id is empty".to_string()));
        }
        let filter = doc! {"group_id":as_ref_to_string(group_id)};
        self.dao.delete(filter).await?;
        Ok(())
    }

    pub async fn find_owner(&self, group_id: impl AsRef<str>) -> Result<GroupMember, AppError> {
        let filter = doc! {"group_id":as_ref_to_string(group_id),"role":GroupRole::Owner.to_string()};
        let result = self.dao.find_one(filter).await?;
        match result {
            Some(member) => Ok(member),
            None => Err(AppError::BizError("Owner not found".to_string())),
        }
    }

    /// 仅添加群管理员
    pub async fn add_admin(
        &self,
        group_id: &str,
        user_id: &str,
    ) -> Result<(), AppError> {
        let member = self.dao.find_one(doc! {
            "group_id": group_id,
            "user_id": user_id,
        }).await?;

        match member {
            Some(m) => {
                if m.role == GroupRole::Owner {
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
                "role": GroupRole::Admin.to_string(),
                "update_time": now(),
            }
        };
        self.dao.update(filter, update).await?;
        Ok(())
    }

    /// 取消群管理员
    pub async fn remove_admin(
        &self,
        group_id: &str,
        user_id: &str,
    ) -> Result<(), AppError> {
        // 查询当前成员
        let member = self.dao.find_one(doc! {
            "group_id": group_id,
            "user_id": user_id,
        }).await?;

        match member {
            Some(m) => {
                match m.role {
                    GroupRole::Owner => {
                        // 群主不能降级
                        return Err(AppError::BizError("group.owner.cannot.downgrade".to_string()));
                    }
                    GroupRole::Member => {
                        // 已经是普通成员，直接 OK
                        return Ok(());
                    }
                    GroupRole::Admin => {
                        // 是管理员，继续往下更新
                    }
                }
            }
            None => {
                // 找不到成员
                return Err(AppError::BizError("group.member.notfound".to_string()));
            }
        }

        // 更新 role => Member
        let filter = doc! {
            "group_id": group_id,
            "user_id": user_id,
        };
        let update = doc! {
            "$set": {
                "role": GroupRole::Member.to_string(),
                "update_time": now(),
            }
        };

        self.dao.update(filter, update).await?;
        Ok(())
    }

    /// 获取单例
    pub fn get() -> Arc<Self> {
        INSTANCE
            .get()
            .expect("INSTANCE is not initialized")
            .clone()
    }
}
static INSTANCE: OnceCell<Arc<GroupMemberService>> = OnceCell::new();