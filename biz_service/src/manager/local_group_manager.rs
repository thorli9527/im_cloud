use crate::entitys::group_entity::GroupInfo;
use crate::entitys::group_member::{GroupMemberMeta, GroupRole};
use crate::manager::common::UserId;
use async_trait::async_trait;
use dashmap::{DashMap, DashSet};
use std::sync::Arc;

pub const SHARD_COUNT: usize = 16;
// === 分片群组结构 ===
#[derive(Debug, Clone)]
pub struct LocalGroupManager {
    pub group_info_map: Arc<DashMap<String, GroupInfo>>,
    pub group_members_shards_map: Arc<Vec<DashMap<String, DashSet<String>>>>,
    pub group_members_meta_map: Arc<Vec<DashMap<String, DashMap<String, GroupMemberMeta>>>>,
    pub user_to_groups_shards: Arc<Vec<DashMap<String, DashSet<String>>>>,
}
#[async_trait]
pub trait LocalGroupManagerOpt: Send + Sync {
    /// 初始化群组
    fn init_group(&self, group_info: GroupInfo);
    ///获取群组信息
    /// # group_id: 群组ID
    fn get_group_info(&self, group_id: &str) -> Option<GroupInfo>;
    ///移除群组
    /// # group_id: 群组ID
    fn remove_group(&self, group_id: &str);
    /// 添加用户到群组
    /// # group_id: 群组ID
    /// # user_id: 用户ID
    /// # alias: 群内昵称
    /// # group_role: 群组角色
    fn add_user(&self, group_id: &str, user_id: &UserId, mute: Option<bool>, alias: &str, group_role: &GroupRole);

    /// 刷新用户信息
    /// # group_id: 群组ID
    /// # user_id: 用户ID
    /// # alias: 群内昵称
    /// # role: 群组角色
    fn refresh_user(&self, group_id: &str, user_id: &UserId, mute: Option<bool>, alias: &Option<String>, role: Option<GroupRole>);
    /// 移除用户从群组
    /// # group_id: 群组ID
    /// # user_id: 用户ID
    fn remove_user(&self, group_id: &str, user_id: &UserId);
    /// 获取群组用户列表
    /// # 返回用户ID列表
    /// # group_id: 群组ID
    fn get_users(&self, group_id: &str) -> Vec<UserId>;
    ///分页获群组用户列表
    /// # 返回用户ID列表
    /// # group_id: 群组ID
    /// # page: 页码，从0开始
    /// # page_size: 每页大小
    fn get_users_page(&self, group_id: &str, page: usize, page_size: usize) -> Vec<UserId>;
    /// 获取在线用户列表
    /// # group_id: 群组ID
    async fn get_online_users(&self, group_id: &str) -> Vec<UserId>;
    /// 获取离线用户列表
    /// # group_id: 群组ID
    async fn get_offline_users(&self, group_id: &str) -> Vec<UserId>;

    async fn get_user_groups(&self, user_id: &str) -> Vec<String>;

    /// 获取用户所在的群组，分页返回
    async fn get_user_groups_page(&self, user_id: &str, page: usize, page_size: usize) -> Vec<String>;

    /// 判断用户是否在群组中
    async fn is_user_in_group(&self, group_id: &str, user_id: &UserId) -> bool;
}
