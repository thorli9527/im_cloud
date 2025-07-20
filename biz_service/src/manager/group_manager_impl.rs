// use crate::biz_service::group_member_service::GroupMemberService;
// use crate::biz_service::group_service::GroupService;
// use crate::manager::group_manager_core::{GroupManager, GroupManagerOpt};
// use crate::manager::user_manager_core::{UserManager, UserManagerOpt};
// use crate::protocol::common::{GroupEntity, GroupMemberEntity, GroupRoleType};
// use anyhow::Result;
// use async_trait::async_trait;
// use common::repository_util::Repository;
// use common::util::date_util::now;
// use common::UserId;
// use deadpool_redis::redis::{cmd, AsyncCommands};
//
// #[async_trait]
// impl GroupManagerOpt for GroupManager {
//     async fn create_group(&self, info: &GroupEntity) -> Result<()> {
//         let group_service = GroupService::get();
//         group_service.dao.insert(info).await?;
//         let mut conn = self.pool.get().await?;
//         let key = Self::key_group_info(&info.id);
//         let json = serde_json::to_string(&info)?;
//
//         let inserted: bool = conn.set_nx(&key, &json).await?;
//         if !inserted {
//             anyhow::bail!("Group already exists: {}", info.id);
//         }
//
//         Ok(())
//     }
//
//     async fn update_group(&self, info: &GroupEntity) -> Result<()> {
//         let group_service = GroupService::get();
//         group_service.dao.save(info).await?;
//         let mut conn = self.pool.get().await?;
//         let key = Self::key_group_info(&info.id);
//
//         // 判断群组是否存在
//         let exists: bool = conn.exists(&key).await?;
//         if !exists {
//             anyhow::bail!("Group does not exist: {}", info.id);
//         }
//
//         // 序列化并写入
//         let json = serde_json::to_string(&info)?;
//         let _: () = conn.set(&key, json).await?;
//
//         Ok(())
//     }
//     async fn dismiss_group(&self, group_id: &str) -> Result<()> {
//         let group_service = GroupService::get();
//         group_service.dismiss_group(group_id).await?;
//         let mut conn = self.pool.get().await?;
//
//         // 1. 删除群组信息键
//         let group_info_key = Self::key_group_info(group_id);
//         let _: () = conn.del(&group_info_key).await?;
//
//         // 2. 删除群成员集合
//         let members_key = Self::key_group_members(group_id);
//         let _: () = conn.del(&members_key).await?;
//
//         // 3. 删除群成员元信息（例如 alias、role、mute 等）
//         let meta_key = Self::key_group_member_meta(group_id);
//         let _: () = conn.del(&meta_key).await?;
//         Ok(())
//     }
//
//     async fn add_user_to_group(
//         &self,
//         group_id: &str,
//         user_id: &UserId,
//         alias: &str,
//         group_role: &GroupRoleType,
//     ) -> Result<()> {
//         let group_member_service = GroupMemberService::get();
//         let now = now() as u64;
//
//         // 构造成员数据
//         let member = GroupMemberEntity {
//             id: format!("{}_{}", group_id, user_id),
//             group_id: group_id.to_string(),
//             uid: user_id.to_string(),
//             role: *group_role as i32,
//             is_muted: false,
//             avatar: alias.clone().to_string(),
//             alias: alias.to_string(),
//             create_time: now,
//             update_time: now,
//         };
//
//         // === Step 1: 持久化到数据库 ===
//         group_member_service.dao.insert(&member).await?;
//
//         // === Step 2: 缓存写入 Redis ===
//         let mut conn = self.pool.get().await?;
//
//         // 2.1 添加成员 ID 到群成员集合
//         let member_set_key = Self::key_group_members(group_id);
//         let _: () = cmd("SADD")
//             .arg(&member_set_key)
//             .arg(user_id)
//             .query_async(&mut conn)
//             .await?;
//
//         // 2.2 写入成员元信息（JSON）到 Redis Hash
//         let meta_json = serde_json::to_string(&member)?;
//         let meta_hash_key = Self::key_group_member_meta(group_id);
//         let _: () = cmd("HSET")
//             .arg(&meta_hash_key)
//             .arg(user_id)
//             .arg(meta_json)
//             .query_async(&mut conn)
//             .await?;
//
//         Ok(())
//     }
//
//     async fn remove_user_from_group(&self, group_id: &str, user_id: &UserId) -> Result<()> {
//         // === Step 1: 删除 MongoDB 中的成员信息 ===
//         let group_member_service = GroupMemberService::get();
//         group_member_service.remove(group_id, user_id).await?;
//
//         // === Step 2: 删除 Redis 中的成员缓存 ===
//         let mut conn = self.pool.get().await?;
//
//         // 2.1 移除群成员集合中的用户 ID
//         let member_set_key = Self::key_group_members(group_id);
//         let _: () = cmd("SREM")
//             .arg(&member_set_key)
//             .arg(&user_id)
//             .query_async(&mut conn)
//             .await?;
//
//         // 2.2 移除成员元信息（Hash 中字段）
//         let meta_key = Self::key_group_member_meta(group_id);
//         let _: () = cmd("HDEL")
//             .arg(&meta_key)
//             .arg(&user_id)
//             .query_async(&mut conn)
//             .await?;
//
//         // === Step 3: 广播退出消息（可选）===
//         // self.broadcast_user_left(group_id, user_id).await?;
//
//         Ok(())
//     }
//
//     async fn group_member_refresh(
//         &self,
//         group_id: &str,
//         user_id: &UserId,
//         mute: Option<bool>,
//         alias: &str,
//         role: &Option<GroupRoleType>,
//     ) -> Result<()> {
//         let mut conn = self.pool.get().await?;
//         let member_set_key = Self::key_group_members(group_id);
//         let meta_key = Self::key_group_member_meta(group_id);
//
//         // Step 1: 确保成员在集合中
//         let _: () = cmd("SADD")
//             .arg(&member_set_key)
//             .arg(&user_id)
//             .query_async(&mut conn)
//             .await?;
//
//         // Step 2: 优先读取 Redis 中元信息
//         let raw: Option<String> = cmd("HGET")
//             .arg(&meta_key)
//             .arg(&user_id)
//             .query_async(&mut conn)
//             .await?;
//
//         let mut meta = if let Some(json) = raw {
//             serde_json::from_str(&json)?
//         } else {
//             // Redis 中没有，尝试从 MongoDB 加载
//             let service = GroupMemberService::get();
//             let from_db = service.find_by_group_id_and_uid(group_id, user_id).await?;
//
//             from_db
//         };
//
//         // Step 3: 更新字段
//         if let Some(r) = role {
//             meta.role = *r as i32;
//         }
//         if let Some(m) = mute {
//             meta.is_muted = m;
//         }
//         meta.alias = alias.to_string();
//         meta.update_time = now() as u64;
//
//         // Step 4: 更新 MongoDB
//         let group_member_service = GroupMemberService::get();
//         group_member_service.dao.save(&meta).await?;
//
//         // Step 5: 写入 Redis 缓存
//         let json = serde_json::to_string(&meta)?;
//         let _: () = cmd("HSET")
//             .arg(&meta_key)
//             .arg(&user_id)
//             .arg(&json)
//             .query_async(&mut conn)
//             .await?;
//
//         Ok(())
//     }
//
//     async fn get_group_info(&self, group_id: &str) -> Result<Option<GroupEntity>> {
//         let mut conn = self.pool.get().await?;
//         let key = Self::key_group_info(group_id);
//
//         // Step 1: 先查 Redis 缓存
//         let value: Option<String> = conn.get(&key).await?;
//         if let Some(json) = value {
//             let entity: GroupEntity = serde_json::from_str(&json)?;
//             return Ok(Some(entity));
//         }
//
//         // Step 2: Redis 未命中，从数据库读取
//         let group_service = GroupService::get();
//         let result = group_service.find_by_group_id(group_id).await?;
//
//         // Step 3: 如果存在，将其写入 Redis 缓存
//         let json = serde_json::to_string(&result)?;
//         let _: () = conn.set(&key, &json).await?;
//
//         Ok(Some(result))
//     }
//
//     async fn get_group_members(&self, group_id: &str) -> Result<Vec<UserId>> {
//         let mut conn = self.pool.get().await?;
//         let key = Self::key_group_members(group_id);
//
//         // Step 1: 尝试从 Redis 获取
//         let members: Vec<UserId> = conn.smembers(&key).await?;
//         if !members.is_empty() {
//             return Ok(members);
//         }
//
//         // Step 2: Redis 中未命中，尝试从数据库加载
//         let member_service = GroupMemberService::get();
//         let entities = member_service.find_by_group_id(group_id).await?;
//
//         let uids: Vec<UserId> = entities.iter().map(|m| m.uid.clone()).collect();
//
//         // Step 3: 回写 Redis 缓存（Set + Hash）
//         if !uids.is_empty() {
//             let _: () = conn.sadd(&key, &uids).await?;
//
//             let meta_key = Self::key_group_member_meta(group_id);
//             for member in &entities {
//                 let json = serde_json::to_string(member)?;
//                 let _: () = conn.hset(&meta_key, &member.uid, json).await?;
//             }
//         }
//
//         Ok(uids)
//     }
//
//
//     async fn is_user_in_group(&self, group_id: &str, user_id: &UserId) -> Result<bool> {
//         // 2. Redis 兜底判断
//         let key = format!("group:member:{}", group_id);
//         let mut conn = self.pool.get().await?;
//         let exists: bool = conn.sismember(&key, user_id).await.unwrap_or(false);
//         Ok(exists)
//     }
//     async fn get_online_group_members(&self, group_id: &str) -> Result<Vec<UserId>> {
//         let members = self.get_group_members(group_id).await?;
//         let user_mgr = UserManager::get();
//
//         let mut result = Vec::with_capacity(members.len());
//         for uid in &members {
//             if user_mgr.is_online(uid).await? {
//                 result.push(uid.clone());
//             }
//         }
//         Ok(result)
//     }
//
//     async fn get_offline_group_members(&self, group_id: &str) -> Result<Vec<UserId>> {
//         let members = self.get_group_members(group_id).await?;
//         let user_mgr = UserManager::get();
//
//         let mut result = Vec::with_capacity(members.len());
//         for uid in &members {
//             if !user_mgr.is_online(uid).await? {
//                 result.push(uid.clone());
//             }
//         }
//
//         Ok(result)
//     }
//
//     async fn get_group_members_by_page(
//         &self,
//         group_id: &str,
//         page: usize,
//         page_size: usize,
//     ) -> Result<Vec<UserId>> {
//         let members = self.get_group_members(group_id).await?;
//         let start = page * page_size;
//         let end = start + page_size;
//         Ok(members.into_iter().skip(start).take(page_size).collect())
//     }
//
//     async fn change_member_role(
//         &self,
//         group_id: &str,
//         user_id: &UserId,
//         role: &GroupRoleType,
//     ) -> Result<()> {
//         use deadpool_redis::redis::cmd;
//         use serde_json;
//
//         let mut conn = self.pool.get().await?;
//         let meta_key = Self::key_group_member_meta(group_id);
//
//         // Step 1: 获取 Redis 中的元信息
//         let raw: Option<String> = cmd("HGET")
//             .arg(&meta_key)
//             .arg(user_id)
//             .query_async(&mut conn)
//             .await?;
//
//         let mut member: GroupMemberEntity = if let Some(json) = raw {
//             serde_json::from_str(&json)?
//         } else {
//             // Redis 未命中 → 查询数据库
//             let db_member = GroupMemberService::get()
//                 .find_by_group_id_and_uid(group_id, user_id)
//                 .await?;
//             db_member
//         };
//
//         // Step 2: 修改角色字段并更新时间
//         member.role = *role as i32;
//         member.update_time = now() as u64;
//
//         // Step 3: 更新 Redis 缓存
//         let updated_json = serde_json::to_string(&member)?;
//         let _: () = cmd("HSET")
//             .arg(&meta_key)
//             .arg(user_id)
//             .arg(&updated_json)
//             .query_async(&mut conn)
//             .await?;
//
//         // Step 4: 更新数据库
//         let group_member_service = GroupMemberService::get();
//         group_member_service
//             .change_member_role(group_id, user_id.as_str(), role)
//             .await?;
//
//         Ok(())
//     }
//
//     async fn find_group_member_by_id(
//         &self,
//         group_id: &str,
//         user_id: &UserId,
//     ) -> Result<Option<GroupMemberEntity>> {
//         let mut conn = self.pool.get().await?;
//         let meta_key = Self::key_group_member_meta(group_id);
//
//         // Step 1: 尝试从 Redis 中读取
//         let raw: Option<String> = cmd("HGET")
//             .arg(&meta_key)
//             .arg(&user_id)
//             .query_async(&mut conn)
//             .await?;
//
//         if let Some(json) = raw {
//             let member: GroupMemberEntity = serde_json::from_str(&json)?;
//             return Ok(Some(member));
//         }
//
//         // Step 2: Redis 未命中，调用服务从数据库加载
//         let member_service = GroupMemberService::get();
//         let member = member_service
//             .find_by_group_id_and_uid(group_id, user_id.as_str())
//             .await?;
//
//         // Step 3: 回写缓存（可选）
//         let json = serde_json::to_string(&member)?;
//         let _: () = cmd("HSET")
//             .arg(&meta_key)
//             .arg(&user_id)
//             .arg(json)
//             .query_async(&mut conn)
//             .await?;
//
//         Ok(Some(member))
//     }
// }
