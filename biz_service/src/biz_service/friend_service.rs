use crate::entitys::friend::FriendEntity;
use crate::manager::user_manager::{UserManager, UserManagerOpt};
use anyhow::Result;
use common::index_trait::MongoIndexModelProvider;
use common::repository_util::{BaseRepository, Repository};
use common::UserId;
use mongodb::bson::Bson;
use mongodb::{bson::doc, Database};
use once_cell::sync::OnceCell;
use std::sync::Arc;

#[derive(Debug)]
pub struct UserFriendService {
    pub dao: BaseRepository<FriendEntity>,
}

impl UserFriendService {
    pub async fn new(db: Database) -> Self {
        let vec = FriendEntity::index_models();

        let owner = Self {
            dao: BaseRepository::new(db, "user_friend").await,
        };

        return owner;
    }

    /// 拉黑好友（将 is_blocked 设置为 true）
    pub async fn friend_block(&self, agent_id: &str, uid: &UserId, friend_id: &UserId) -> Result<()> {
        let filter = doc! {
            "agent_id": agent_id,
            "uid": uid,
            "friend_id": friend_id
        };

        let update = doc! {
            "$set": {
                "is_blocked": Bson::Boolean(true)
            }
        };
        self.dao.update(filter, update).await?;
        Ok(())
    }

    /// 解除拉黑好友（将 is_blocked 设置为 false）
    pub async fn friend_unblock(&self, agent_id: &str, uid: &UserId, friend_id: &UserId) -> Result<()> {
        let filter = doc! {
            "agent_id": agent_id,
            "uid": uid,
            "friend_id": friend_id
        };

        let update = doc! {
            "$set": {
                "is_blocked": Bson::Boolean(false)
            }
        };

        self.dao.update(filter, update).await?;
        Ok(())
    }

    /// 是否是好友（Mongo 或缓存判断）
    pub async fn is_friend(&self, uid: &UserId, friend_id: &UserId) -> Result<bool> {
        let manager = UserManager::get();
        manager.is_friend(uid, friend_id).await
    }

    /// 获取好友列表
    pub async fn get_friend_list(&self, agent_id: &str, uid: &UserId) -> Result<Vec<FriendEntity>> {
        let filter = doc! {
            "agent_id": agent_id,
            "uid": uid,
        };
        let list = self.dao.query(filter).await?;
        Ok(list)
    }

    /// 查询单个好友详细信息（例如备注/昵称）
    pub async fn get_friend_detail(&self, uid: &UserId, friend_id: &UserId) -> Result<Option<FriendEntity>> {
        let filter = doc! {
            "uid": uid,
            "friend_id": friend_id
        };
        let result = self.dao.find_one(filter).await?;
        return Ok(result);
    }

    /// 批量删除某用户相关记录（如注销）
    pub async fn delete_all_for_user(&self, uid: &UserId) -> Result<()> {
        let filter = doc! {
            "uid": uid
        };
        self.dao.delete(filter).await?;
        Ok(())
    }

    pub async fn init(db: Database) {
        let instance = Self::new(db).await;
        INSTANCE.set(Arc::new(instance)).expect("UserFriendService already initialized");
    }

    pub fn get() -> Arc<Self> {
        INSTANCE.get().expect("UserFriendService is not initialized").clone()
    }
}

static INSTANCE: OnceCell<Arc<UserFriendService>> = OnceCell::new();
