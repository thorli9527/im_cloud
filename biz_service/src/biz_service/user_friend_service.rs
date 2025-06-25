use crate::entitys::user_friend::{FriendSourceType, FriendStatus, UserFriend};
use crate::manager::common::UserId;
use crate::manager::user_redis_manager::{UserManager, UserManagerOpt};
use anyhow::{Context, Result};
use common::repository_util::{BaseRepository, Repository};
use mongodb::{Database, bson::doc};
use once_cell::sync::OnceCell;
use std::sync::Arc;

#[derive(Debug)]
pub struct UserFriendService {
    pub dao: BaseRepository<UserFriend>,
}

impl UserFriendService {
    pub fn new(db: Database) -> Self {
        let collection = db.collection("user_friend");
        Self { dao: BaseRepository::new(db, collection.clone()) }
    }

    /// 添加好友（可配置昵称/来源等）
    pub async fn add_friend(&self, agent_id: &str, uid: &UserId, friend_id: &UserId, nickname: Option<String>, source_type: FriendSourceType) -> Result<()> {
        // 校验对方存在
        let client_opt = UserManager::get().get_user_info(agent_id, friend_id).await?;
        if client_opt.is_none() {
            return Err(anyhow::anyhow!("用户不存在"));
        }

        // 检查是否已存在好友记录
        let filter = doc! {
            "agent_id": agent_id,
            "uid": uid,
            "friend_id": friend_id
        };
        let exists = self.dao.find_one(filter).await?;
        if exists.is_some() {
            return Ok(()); // 已存在，忽略重复添加
        }

        let friend = UserFriend {
            id: uuid::Uuid::new_v4().to_string(),
            agent_id: agent_id.to_string(),
            uid: uid.to_string(),
            friend_id: friend_id.to_string(),
            nickname,
            remark: None,
            source_type,
            friend_status: FriendStatus::Accepted,
            created_at: common::util::date_util::now(),
        };

        self.dao.insert(&friend).await?;

        // 更新 Redis 缓存
        UserManager::get().add_friend(agent_id, uid, friend_id).await?;

        Ok(())
    }

    /// 删除好友
    pub async fn remove_friend(&self, agent_id: &str, uid: &UserId, friend_id: &UserId) -> Result<()> {
        let filter = doc! {
            "agent_id": agent_id,
            "uid": uid,
            "friend_id": friend_id
        };

        self.dao.delete(filter).await?;
        UserManager::get().remove_friend(agent_id, uid, friend_id).await?;

        Ok(())
    }

    /// 是否是好友（Mongo 或缓存判断）
    pub async fn is_friend(&self, agent_id: &str, uid: &UserId, friend_id: &UserId) -> Result<bool> {
        let manager = UserManager::get();
        manager.is_friend(agent_id, uid, friend_id).await
    }

    /// 获取好友列表
    pub async fn get_friend_list(&self, agent_id: &str, uid: &UserId) -> Result<Vec<UserFriend>> {
        let filter = doc! {
            "agent_id": agent_id,
            "uid": uid,
            "friend_status": FriendStatus::Accepted as i32
        };
        let list = self.dao.query(filter).await?;
        Ok(list)
    }

    /// 查询单个好友详细信息（例如备注/昵称）
    pub async fn get_friend_detail(&self, agent_id: &str, uid: &UserId, friend_id: &UserId) -> Result<Option<UserFriend>> {
        let filter = doc! {
            "agent_id": agent_id,
            "uid": uid,
            "friend_id": friend_id
        };
        let result = self.dao.find_one(filter).await?;
        return Ok(result);
    }

    /// 批量删除某用户相关记录（如注销）
    pub async fn delete_all_for_user(&self, agent_id: &str, uid: &UserId) -> Result<()> {
        let filter = doc! {
            "agent_id": agent_id,   //
            "uid": uid 
        };
        self.dao.delete(filter).await?;
        Ok(())
    }

    pub fn init(db: Database) {
        let instance = Self::new(db);
        INSTANCE.set(Arc::new(instance)).expect("UserFriendService already initialized");
    }

    pub fn get() -> Arc<Self> {
        INSTANCE.get().expect("UserFriendService is not initialized").clone()
    }
}

static INSTANCE: OnceCell<Arc<UserFriendService>> = OnceCell::new();
