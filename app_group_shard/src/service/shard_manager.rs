use crate::db::hash_shard_map::HashShardMap;
use anyhow::Result;
use arc_swap::ArcSwap;
use async_trait::async_trait;
use biz_service::protocol::common::GroupRoleType;
use biz_service::protocol::rpc::arb_models::{MemberRef, ShardState};
use common::config::ShardConfig;
use common::{GroupId, UserId};
use std::hash::Hash;
use std::sync::Arc;
use tokio::sync::RwLock;

pub const GROUP_SHARD_SIZE: usize = 64;
pub const MEMBER_SHARD_SIZE: usize = 16;
#[derive(Debug, Default, Clone)]
pub struct ShardInfo {
    pub state: ShardState,
    pub last_update_time: u64,
    pub last_heartbeat: u64,
}
#[derive(Debug)]
pub struct MemData {
    /// 分片映射，存储群组成员信息
    pub shard_map: HashShardMap,
    /// 在线群组成员映射，存储群组成员信息
    pub shard_info: RwLock<ShardInfo>,
}
impl Clone for MemData {
    fn clone(&self) -> Self {
        Self {
            shard_map: self.shard_map.clone(),
            shard_info: RwLock::new(ShardInfo::default()),
        }
    }
}

impl MemData {
    pub fn new() -> Self {
        Self {
            shard_map: HashShardMap::new(GROUP_SHARD_SIZE, MEMBER_SHARD_SIZE),
            shard_info: RwLock::new(ShardInfo::default()),
        }
    }
}
#[derive(Debug)]
pub struct ShardManager {
    pub snapshot: ArcSwap<MemData>,
    pub current: ArcSwap<MemData>,
    pub shard_config: ShardConfig,
}

#[async_trait]
pub trait ShardManagerOpt: Send + Sync {
    async fn load_from_data(&self) -> anyhow::Result<()>;
    ///创建群组
    async fn create(&self, group_id: &str) -> anyhow::Result<()>;
    ///删除群组
    fn dismiss(&self, group_id: &str);
    /// 添加用户到指定群组（自动根据 group_id 映射分片）
    fn add_member(&self, group_id: &str, uid: &UserId, role: GroupRoleType) -> anyhow::Result<()>;
    /// 移除用户从指定群组（自动根据 group_id 映射分片）
    fn remove_member(&self, group_id: &GroupId, uid: &UserId) -> anyhow::Result<()>;
    /// 获取某个群组的所有成员 ID 列表
    fn get_member(&self, group_id: &GroupId) -> Result<Vec<MemberRef>>;
    /// 获取群组成员分页列表
    fn get_member_page(&self, group_id: &GroupId, offset: usize, limit: usize) -> Result<Option<Vec<MemberRef>>>;
    fn get_member_count(&self, group_id: &GroupId) -> Result<usize>;
    /// 标记用户在线
    fn online(&self, group_id: &GroupId, uid: &UserId) -> Result<()>;
    /// 移除在线用户
    fn offline(&self, group_id: &GroupId, uid: &UserId);
    /// 获取某个群组的所有在线用户
    fn get_on_line_member(&self, group_id: &GroupId) -> Vec<UserId>;
    ///修改角色
    fn change_role(&self, group_id: &GroupId, uid: &UserId, role: GroupRoleType) -> Result<()>;
    /// 获取用户所在的群组
    fn get_user_groups(&self, uid: &UserId) -> anyhow::Result<Vec<Arc<str>>>;
    /// 获取在线管理员
    async fn get_admin_member(&self, group_id: &GroupId) -> Result<Option<Vec<UserId>>>;
}
