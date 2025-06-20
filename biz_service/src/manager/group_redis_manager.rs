use crate::entitys::group_entity::GroupInfo;
use crate::manager::common::UserId;
use crate::manager::local_group_manager::{LocalGroupManager, LocalGroupManagerOpt};
use crate::manager::user_redis_manager::{UserManager, UserManagerOpt};
use anyhow::Result;
use async_trait::async_trait;
use dashmap::DashSet;
use deadpool_redis::{redis::{cmd, AsyncCommands}, Pool as RedisPool};
use once_cell::sync::OnceCell;
use std::sync::Arc;

/// 群组管理器
#[derive(Debug, Clone)]
pub struct GroupManager {
    pool: RedisPool,
    local_group_manager: Arc<LocalGroupManager>,
    use_local_cache: bool,
}

impl GroupManager {
    pub fn new(pool: RedisPool, use_local_cache: bool) -> Self {
        let local_group_manager = LocalGroupManager::get();
        let instance = Self {
            pool,
            local_group_manager,
            use_local_cache,
        };
        GroupManager::init(instance.clone());
        instance
    }

    pub fn init(instance: Self) {
        INSTANCE.set(Arc::new(instance)).expect("GroupManager already initialized");
    }

    pub fn get() -> Arc<Self> {
        INSTANCE.get().expect("GroupManager not initialized").clone()
    }

    fn key_group_info(group_id: &str) -> String {
        format!("group:info:{}", group_id)
    }

    fn key_group_members(group_id: &str) -> String {
        format!("group:member:{}", group_id)
    }
}

static INSTANCE: OnceCell<Arc<GroupManager>> = OnceCell::new();

/// 群组操作接口
#[async_trait]
pub trait GroupManagerOpt: Send + Sync {
    async fn create_group(&self, info: GroupInfo) -> Result<()>;
    async fn dismiss_group(&self, group_id: &str) -> Result<()>;
    async fn add_user_to_group(&self, group_id: &str, user_id: &UserId) -> Result<()>;
    async fn remove_user_from_group(&self, group_id: &str, user_id: &UserId) -> Result<()>;
    async fn get_group_info(&self, group_id: &str) -> Result<Option<GroupInfo>>;
    async fn get_group_members(&self, group_id: &str) -> Result<Vec<UserId>>;
    ///判断用户是否在群组中
    /// # group_id: 群组ID
    /// # user_id: 用户ID
    /// Returns true if the user is a member of the group, false otherwise.
    async fn is_user_in_group(&self, group_id: &str, user_id: &UserId) -> Result<bool>;
    async fn get_online_group_members(&self, group_id: &str) -> Result<Vec<UserId>>;
    async fn get_offline_group_members(&self, group_id: &str) -> Result<Vec<UserId>>;
    async fn get_group_members_by_page(
        &self,
        group_id: &str,
        page: usize,
        page_size: usize,
    ) -> Result<Vec<UserId>> ;
}

#[async_trait]
impl GroupManagerOpt for GroupManager {
    async fn create_group(&self, info: GroupInfo) -> Result<()> {
        let mut conn = self.pool.get().await?;
        let key = Self::key_group_info(&info.id);
        let json = serde_json::to_string(&info)?;

        let inserted: bool = conn.set_nx(&key, &json).await?;
        if !inserted {
            anyhow::bail!("Group already exists: {}", info.id);
        }

        if self.use_local_cache {
            self.local_group_manager.init_group(info);
        }

        Ok(())
    }
    
    async fn dismiss_group(&self, group_id: &str) -> Result<()> {
        let mut conn = self.pool.get().await?;
        let key = Self::key_group_info(group_id);

        let _: () = conn.del(&key).await?;

        if self.use_local_cache {
            self.local_group_manager.remove_group(group_id);
        }

        // 删除群成员
        let members_key = Self::key_group_members(group_id);
        let _: () = conn.del(&members_key).await?;

        Ok(())
    }

    async fn add_user_to_group(&self, group_id: &str, user_id: &UserId) -> Result<()> {
        let mut conn = self.pool.get().await?;
        let key = Self::key_group_members(group_id);

        let _: () = cmd("SADD").arg(&key).arg(&user_id).query_async(&mut conn).await?;

        if self.use_local_cache {
            self.local_group_manager.add_user(group_id, &user_id);
        }

        Ok(())
    }

    async fn remove_user_from_group(&self, group_id: &str, user_id: &UserId) -> Result<()> {
        let mut conn = self.pool.get().await?;
        let key = Self::key_group_members(group_id);

        let _: () = cmd("SREM").arg(&key).arg(&user_id).query_async(&mut conn).await?;

        if self.use_local_cache {
            self.local_group_manager.remove_user(group_id, &user_id);
        }
        //发消息
        Ok(())
    }

    async fn get_group_info(&self, group_id: &str) -> Result<Option<GroupInfo>> {
        if self.use_local_cache {
            if let Some(info) = self.local_group_manager.get_group_info(group_id) {
                return Ok(Some(info));
            }
        }

        let mut conn = self.pool.get().await?;
        let key = Self::key_group_info(group_id);
        let json: Option<String> = conn.get(&key).await?;

        Ok(match json {
            Some(val) => Some(serde_json::from_str(&val)?),
            None => None,
        })
    }

    async fn get_group_members(&self, group_id: &str) -> Result<Vec<UserId>> {
        if self.use_local_cache {
            return Ok(self.local_group_manager.get_users(group_id));
        }

        let mut conn = self.pool.get().await?;
        let key = Self::key_group_members(group_id);
        let members: Vec<UserId> = conn.smembers(&key).await?;
        Ok(members)
    }

    async fn get_online_group_members(&self, group_id: &str) -> Result<Vec<UserId>> {
        let agent_id = self
            .get_group_info(group_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Group not found: {}", group_id))?
            .agent_id;

        let members = self.get_group_members(group_id).await?;
        let user_mgr = UserManager::get();

        let mut result = Vec::with_capacity(members.len());
        for uid in &members {
            if user_mgr.is_online(&agent_id, uid).await? {
                result.push(uid.clone());
            }
        }

        Ok(result)
    }

    async fn get_offline_group_members(&self, group_id: &str) -> Result<Vec<UserId>> {
        let agent_id = self
            .get_group_info(group_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Group not found: {}", group_id))?
            .agent_id;

        let members = self.get_group_members(group_id).await?;
        let user_mgr = UserManager::get();

        let mut result = Vec::with_capacity(members.len());
        for uid in &members {
            if !user_mgr.is_online(&agent_id, uid).await? {
                result.push(uid.clone());
            }
        }

        Ok(result)
    }
    async  fn get_group_members_by_page(
        &self,
        group_id: &str,
        page: usize,
        page_size: usize,
    ) -> Result<Vec<UserId>> {
        let members = self.get_group_members(group_id).await?;
        let start = page * page_size;
        let end = start + page_size;
        Ok(members.into_iter().skip(start).take(page_size).collect())
    }


    async fn is_user_in_group(&self, group_id: &str, user_id: &UserId) -> Result<bool> {
        // 2. Redis 兜底判断
        let key = format!("group:member:{}", group_id);
        let mut conn = self.pool.get().await?;
        let exists: bool = conn.sismember(&key, user_id).await.unwrap_or(false);
        // 3. 命中 Redis 后写入本地缓存，避免下次重复查询
        if exists {
            let shard = self.local_group_manager.get_group_members_shard(group_id);
            shard
                .entry(group_id.to_string())
                .or_insert_with(DashSet::new)
                .insert(user_id.to_string());
        }
        Ok(exists)
    }

}
