use crate::entitys::group_entity::GroupInfo;
use crate::entitys::group_member::{GroupMemberMeta, GroupRole};
use crate::manager::common::UserId;
use crate::manager::local_group_manager::{LocalGroupManager, LocalGroupManagerOpt};
use crate::manager::user_manager_core::{UserManager, UserManagerOpt};
use anyhow::Result;
use async_trait::async_trait;
use dashmap::DashSet;
use deadpool_redis::{
    redis::{cmd, AsyncCommands}, Pool as RedisPool,
    Pool,
};
use once_cell::sync::OnceCell;
use std::sync::Arc;

/// 群组管理器
#[derive(Debug, Clone)]
pub struct GroupManager {
    pub pool: RedisPool,
    pub local_group_manager: Arc<LocalGroupManager>,
    pub use_local_cache: bool,
}

impl GroupManager {
    fn new(pool: RedisPool, use_local_cache: bool) -> Self {
        let local_group_manager = LocalGroupManager::get();
        Self { pool, local_group_manager, use_local_cache }
    }


    pub fn init(pool: Pool, use_local_cache: bool) {
        let instance = Self::new(pool,use_local_cache);
        INSTANCE.set(Arc::new(instance)).expect("AgentService already initialized");
    }

    pub fn get() -> Arc<Self> {
        INSTANCE.get().expect("GroupManager not initialized").clone()
    }

    pub fn key_group_info(group_id: &str) -> String {
        format!("group:info:{}", group_id)
    }

    pub fn key_group_members(group_id: &str) -> String {
        format!("group:member:{}", group_id)
    }
    pub fn key_group_member_meta(group_id: &str) -> String {
        format!("group:member:meta:{}", group_id)
    }
}

static INSTANCE: OnceCell<Arc<GroupManager>> = OnceCell::new();

/// 群组操作接口
#[async_trait]
pub trait GroupManagerOpt: Send + Sync {
    async fn create_group(&self, info: GroupInfo) -> Result<()>;
    async fn dismiss_group(&self, group_id: &str) -> Result<()>;
    async fn add_user_to_group(&self, group_id: &str, user_id: &UserId, mute: Option<bool>, alian: &str, group_role: &GroupRole) -> Result<()>;
    async fn remove_user_from_group(&self, group_id: &str, user_id: &UserId) -> Result<()>;
    async fn group_member_refresh(&self, group_id: &str, user_id: &UserId, mute: Option<bool>, alias: &Option<String>, role: &Option<GroupRole>) -> Result<()>;
    async fn get_group_info(&self, group_id: &str) -> Result<Option<GroupInfo>>;
    async fn get_group_members(&self, group_id: &str) -> Result<Vec<UserId>>;
    ///判断用户是否在群组中
    /// # group_id: 群组ID
    /// # user_id: 用户ID
    /// Returns true if the user is a member of the group, false otherwise.
    async fn is_user_in_group(&self, group_id: &str, user_id: &UserId) -> Result<bool>;
    async fn get_online_group_members(&self, group_id: &str) -> Result<Vec<UserId>>;
    async fn get_offline_group_members(&self, group_id: &str) -> Result<Vec<UserId>>;
    async fn get_group_members_by_page(&self, group_id: &str, page: usize, page_size: usize) -> Result<Vec<UserId>>;
}