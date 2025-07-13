use crate::entitys::group_entity::{GroupEntity};
use crate::entitys::group_member::{GroupMemberMeta, GroupRole};
use crate::manager::group_manager_core::{GroupManager, GroupManagerOpt};
use crate::manager::user_manager_core::{UserManager, UserManagerOpt};
use anyhow::Result;
use async_trait::async_trait;
use dashmap::DashSet;
use deadpool_redis::redis::{cmd, AsyncCommands};
use common::UserId;

#[async_trait]
impl GroupManagerOpt for GroupManager {
    async fn create_group(&self, info: &GroupEntity) -> Result<()> {
        let mut conn = self.pool.get().await?;
        let key = Self::key_group_info(&info.id);
        let json = serde_json::to_string(&info)?;

        let inserted: bool = conn.set_nx(&key, &json).await?;
        if !inserted {
            anyhow::bail!("Group already exists: {}", info.id);
        }

        Ok(())
    }

    async fn dismiss_group(&self, group_id: &str) -> Result<()> {
        let mut conn = self.pool.get().await?;

        // 1. 删除群组信息键
        let group_info_key = Self::key_group_info(group_id);
        let _: () = conn.del(&group_info_key).await?;

        // 2. 删除群成员集合
        let members_key = Self::key_group_members(group_id);
        let _: () = conn.del(&members_key).await?;

        // 3. 删除群成员元信息（例如 alias、role、mute 等）
        let meta_key = Self::key_group_member_meta(group_id);
        let _: () = conn.del(&meta_key).await?;
        Ok(())
    }

    async fn add_user_to_group(&self, group_id: &str, user_id: &UserId, mute: Option<bool>, alias: &str, group_role: &GroupRole) -> Result<()> {

        let mut conn = self.pool.get().await?;

        // 1. 添加成员 ID 到成员集合
        let member_set_key = Self::key_group_members(group_id);
        let _: () = cmd("SADD").arg(&member_set_key).arg(user_id).query_async(&mut conn).await?;

        // 2. 写入元信息 JSON 到成员元信息 Hash
        let meta = GroupMemberMeta {
            id: format!("{}_{}", group_id, user_id),
            group_id: group_id.to_string(),
            uid: user_id.to_string(),
            role: group_role.clone(),
            alias: Some(alias.to_string()),
            mute: mute.unwrap_or(false),
        };
        let json = serde_json::to_string(&meta)?;
        let meta_hash_key = Self::key_group_member_meta(group_id);
        let _: () = cmd("HSET").arg(&meta_hash_key).arg(user_id).arg(json).query_async(&mut conn).await?;

        Ok(())
    }
    async fn group_member_refresh(&self, group_id: &str, user_id: &UserId, mute: Option<bool>, alias: &Option<String>, role: &Option<GroupRole>) -> Result<()> {
        let mut conn = self.pool.get().await?;
        let key = Self::key_group_members(group_id);
        let meta_key = Self::key_group_member_meta(group_id);

        // 1. 保证成员存在于群组集合
        let _: () = cmd("SADD").arg(&key).arg(&user_id).query_async(&mut conn).await?;

        // 2. 读取原有 meta 并更新
        let raw: Option<String> = cmd("HGET").arg(&meta_key).arg(&user_id).query_async(&mut conn).await?;

        if let Some(json) = raw {
            let mut meta: GroupMemberMeta = serde_json::from_str(&json)?;

            if let Some(new_alias) = alias {
                meta.alias = Some(new_alias.clone());
            }

            if let Some(new_role) = role {
                meta.role = new_role.clone();
            }

            if let Some(mute_flag) = mute {
                meta.mute = mute_flag;
            }

            let updated = serde_json::to_string(&meta)?;
            let _: () = cmd("HSET").arg(&meta_key).arg(&user_id).arg(&updated).query_async(&mut conn).await?;
        }

        Ok(())
    }

    async fn remove_user_from_group(&self, group_id: &str, user_id: &UserId) -> Result<()> {
        let mut conn = self.pool.get().await?;

        // Step 1: 移除 Redis 中的成员关系
        let key = Self::key_group_members(group_id);
        let _: () = cmd("SREM").arg(&key).arg(&user_id).query_async(&mut conn).await?;

        // Step 2: 移除 Redis 中的成员元数据
        let meta_key = Self::key_group_member_meta(group_id);
        let _: () = cmd("HDEL").arg(&meta_key).arg(&user_id).query_async(&mut conn).await?;

        // Step 4: 广播用户退出消息（可选）
        // self.broadcast_user_left(group_id, user_id).await?;

        Ok(())
    }

    async fn get_group_info(&self, group_id: &str) -> Result<Option<GroupEntity>> {

        let mut conn = self.pool.get().await?;
        let key = Self::key_group_info(group_id);
        let json: Option<String> = conn.get(&key).await?;

        Ok(match json {
            Some(val) => Some(serde_json::from_str(&val)?),
            None => None,
        })
    }

    async fn get_group_members(&self, group_id: &str) -> Result<Vec<UserId>> {

        let mut conn = self.pool.get().await?;
        let key = Self::key_group_members(group_id);
        let members: Vec<UserId> = conn.smembers(&key).await?;
        Ok(members)
    }

    async fn get_online_group_members(&self, group_id: &str) -> Result<Vec<UserId>> {
        let members = self.get_group_members(group_id).await?;
        let user_mgr = UserManager::get();

        let mut result = Vec::with_capacity(members.len());
        for uid in &members {
            if user_mgr.is_online( uid).await? {
                result.push(uid.clone());
            }
        }
        Ok(result)
    }

    async fn get_offline_group_members(&self, group_id: &str) -> Result<Vec<UserId>> {

        let members = self.get_group_members(group_id).await?;
        let user_mgr = UserManager::get();

        let mut result = Vec::with_capacity(members.len());
        for uid in &members {
            if !user_mgr.is_online( uid).await? {
                result.push(uid.clone());
            }
        }

        Ok(result)
    }
    async fn get_group_members_by_page(&self, group_id: &str, page: usize, page_size: usize) -> Result<Vec<UserId>> {
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
        Ok(exists)
    }

    async fn update_group(&self, info: &GroupEntity) -> Result<()> {
        todo!()
    }
}
