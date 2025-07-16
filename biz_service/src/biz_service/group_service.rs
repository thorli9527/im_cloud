use crate::biz_service::group_member_service::GroupMemberService;
use crate::biz_service::user_service::UserService;
use crate::protocol::common::{GroupEntity, GroupMemberEntity, GroupRoleType};
use anyhow::{anyhow, Result};
use common::errors::AppError;
use common::repository_util::{BaseRepository, Repository};
use common::util::common_utils::as_ref_to_string;
use common::util::date_util::now;
use mongodb::bson::doc;
use mongodb::Database;
use once_cell::sync::OnceCell;
use std::sync::Arc;

#[derive(Debug)]
pub struct GroupService {
    pub dao: BaseRepository<GroupEntity>,
}

impl GroupService {
    pub fn new(db: Database) -> Self {
        let collection = db.collection("group_info");
        Self {
            dao: BaseRepository::new(db, collection.clone()),
        }
    }

    pub async fn find_by_group_id(&self, group_id: impl AsRef<str>) -> Result<GroupEntity, AppError> {
        let result = self
            .dao
            .find_by_id(group_id.as_ref())
            .await?;
        match result {
            Some(group) => Ok(group),
            None => Err(AppError::BizError("Group not found".to_string())),
        }
    }

    pub async fn create_group(
        &self,
        group: &GroupEntity,
        members: &Vec<String>,
    ) -> Result<()> {
        // ✅ 3. 创建群组
        let user_service = UserService::get();
        let user_info=user_service.dao.find_by_id(group.owner_id.as_str()).await?;
        if user_info.is_none() {
            return Err(anyhow!("user.not.found").into());
        }
        let user=user_info.unwrap();
        self.dao.insert(group).await?;
        let group_member_service = GroupMemberService::get();
        let now = now() as u64;
        // ✅ 4. 添加群主
        let owner_id = group.owner_id.clone();
        let owner = GroupMemberEntity {
            id: "".to_string(),
            group_id: group.id.to_string(),
            uid: owner_id,
            role: GroupRoleType::Owner as i32,
            is_muted: false,
            alias: user.user_name.clone(),
            create_time: now,
            update_time: now,
            avatar: "".to_string(),
        };
        group_member_service.dao.insert(&owner).await?;

        // ✅ 5. 添加其他初始成员
        for user_id in members {
            if user_id == &group.owner_id {
                continue; // 跳过群主
            }
            let member = GroupMemberEntity {
                id: "".to_string(),
                group_id: group.id.to_string(),
                uid: user_id.clone(),
                role: GroupRoleType::Member as i32,
                alias: user.user_name.clone(),
                is_muted: false,
                avatar: "".to_string(),
                create_time: now,
                update_time: now,
            };
            group_member_service.dao.insert(&member).await?;
        }
        Ok(())
    }

    /// 解散群组
    pub async fn dismiss_group(&self, group_id: &str) -> Result<(), AppError> {
        // 2. 删除群成员信息
        let group_member_service = GroupMemberService::get();
        group_member_service
            .dao
            .delete_by_id(group_id)
            .await?;
        // 3. 删除群组本体
        self.dao.delete_by_id(group_id).await?;
        Ok(())
    }
    ///修改群名称
    pub async fn update_group_name(
        &self,
        group_id: impl AsRef<str>,
        new_name: impl AsRef<str>,
    ) -> Result<(), AppError> {
        let filter = doc! {"group_id": as_ref_to_string(&group_id)};
        let update = doc! {
            "$set": {
                "name": as_ref_to_string(&new_name),
                "update_time": common::util::date_util::now(),
            }
        };
        self.dao.update(filter, update).await?;
        Ok(())
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
        INSTANCE.get().expect("INSTANCE is not initialized").clone()
    }
}
static INSTANCE: OnceCell<Arc<GroupService>> = OnceCell::new();
