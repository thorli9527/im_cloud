// use crate::protocol::common::{GroupEntity, GroupMemberEntity, GroupRoleType};
// use anyhow::Result;
// use async_trait::async_trait;
// use common::redis::redis_pool::RedisPoolTools;
// use common::{RedisPool, UserId};
// use deadpool_redis::redis::AsyncCommands;
// use once_cell::sync::OnceCell;
// use std::sync::Arc;
//
// /// 群组管理器
// #[derive(Debug, Clone)]
// pub struct GroupManager {
//     pub pool: Arc<RedisPool>,
// }
//
// impl GroupManager {
//     fn new(pool: Arc<RedisPool>) -> Self {
//         Self { pool }
//     }
//
//     pub fn init() {
//         let pool = RedisPoolTools::get().clone();
//         let instance = Self::new(pool);
//         INSTANCE.set(Arc::new(instance)).expect("AgentService already initialized");
//     }
//
//     pub fn get() -> Arc<Self> {
//         INSTANCE.get().expect("GroupManager not initialized").clone()
//     }
//
//     pub fn key_group_info(group_id: &str) -> String {
//         format!("group:info:{}", group_id)
//     }
//
//     pub fn key_group_members(group_id: &str) -> String {
//         format!("group:member:{}", group_id)
//     }
//     pub fn key_group_member_meta(group_id: &str) -> String {
//         format!("group:member:meta:{}", group_id)
//     }
// }
//
// static INSTANCE: OnceCell<Arc<GroupManager>> = OnceCell::new();
//
// /// 群组操作接口
// /// 群组管理功能抽象接口，支持创建、成员管理、分页查询、在线状态判断等功能。
// #[async_trait]
// pub trait GroupManagerOpt: Send + Sync {
//     /// 创建一个群组，持久化群组信息。
//     ///
//     /// # Arguments
//     /// * `info` - 群组的详细信息结构体。
//     async fn create_group(&self, info: &GroupEntity) -> Result<()>;
//
//     async fn update_group(&self, info: &GroupEntity) -> Result<()>;
//
//     /// 解散一个群组并清理相关资源。
//     ///
//     /// # Arguments
//     /// * `group_id` - 要解散的群组 ID。
//     async fn dismiss_group(&self, group_id: &str) -> Result<()>;
//
//     /// 将一个用户添加到指定群组。
//     ///
//     /// # Arguments
//     /// * `group_id` - 目标群组 ID。
//     /// * `user_id` - 待添加的用户 ID。
//     /// * `mute` - 是否禁言该用户（可选）。
//     /// * `alian` - 用户在群内的备注名（昵称）。
//     /// * `group_role` - 用户在群中的角色（成员、管理员等）。
//     async fn add_user_to_group(&self, group_id: &str, user_id: &UserId,  alian: &str, group_role: &GroupRoleType) -> Result<()>;
//
//     /// 将一个用户从群组中移除。
//     ///
//     /// # Arguments
//     /// * `group_id` - 目标群组 ID。
//     /// * `user_id` - 要移除的用户 ID。
//     async fn remove_user_from_group(&self, group_id: &str, user_id: &UserId) -> Result<()>;
//
//     /// 刷新指定用户在群组中的成员信息（如禁言、备注名、角色）。
//     ///
//     /// # Arguments
//     /// * `group_id` - 群组 ID。
//     /// * `user_id` - 目标用户 ID。
//     /// * `mute` - 是否禁言（可选）。
//     /// * `alias` - 用户新的群昵称（可选）。
//     /// * `role` - 用户新的群角色（可选）。
//     async fn group_member_refresh(&self, group_id: &str, user_id: &UserId, mute: Option<bool>, alias: &str, role: &Option<GroupRoleType>) -> Result<()>;
//
//     /// 获取指定群组的详细信息。
//     ///
//     /// # Arguments
//     /// * `group_id` - 群组 ID。
//     ///
//     /// # Returns
//     /// * `Option<GroupEntity>` - 如果群存在则返回群信息，否则为 None。
//     async fn get_group_info(&self, group_id: &str) -> Result<Option<GroupEntity>>;
//
//     /// 获取指定群组中所有成员的用户 ID 列表。
//     ///
//     /// # Arguments
//     /// * `group_id` - 群组 ID。
//     async fn get_group_members(&self, group_id: &str) -> Result<Vec<UserId>>;
//
//     /// 判断某个用户是否属于指定群组。
//     ///
//     /// # Arguments
//     /// * `group_id` - 群组 ID。
//     /// * `user_id` - 用户 ID。
//     ///
//     /// # Returns
//     /// * `true` 表示用户在群组中，`false` 表示不在。
//     async fn is_user_in_group(&self, group_id: &str, user_id: &UserId) -> Result<bool>;
//
//     /// 获取群组中在线状态的用户 ID 列表。
//     ///
//     /// # Arguments
//     /// * `group_id` - 群组 ID。
//     async fn get_online_group_members(&self, group_id: &str) -> Result<Vec<UserId>>;
//
//     /// 获取群组中离线状态的用户 ID 列表。
//     ///
//     /// # Arguments
//     /// * `group_id` - 群组 ID。
//     async fn get_offline_group_members(&self, group_id: &str) -> Result<Vec<UserId>>;
//
//     /// 分页获取群组成员 ID 列表。
//     ///
//     /// # Arguments
//     /// * `group_id` - 群组 ID。
//     /// * `page` - 页码，从 0 开始。
//     /// * `page_size` - 每页成员数量。
//     async fn get_group_members_by_page(&self, group_id: &str, page: usize, page_size: usize) -> Result<Vec<UserId>>;
//
//     /// 修改群组成员角色。
//     async fn change_member_role(&self, group_id: &str, user_id: &UserId, role: &GroupRoleType)-> Result<()>;
//
//     /// 获取群组成员信息。
//     async fn find_group_member_by_id(&self, group_id: &str, user_id: &UserId) -> Result<Option<GroupMemberEntity>>;
// }
