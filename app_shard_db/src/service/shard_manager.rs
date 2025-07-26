use crate::db::hash_shard_map::HashShardMap;
use anyhow::Result;
use arc_swap::ArcSwap;
use async_trait::async_trait;
use biz_service::protocol::arb::rpc_arb_group::arb_group_service_client::ArbGroupServiceClient;
use biz_service::protocol::arb::rpc_arb_models::{MemberRef, ShardState};
use common::config::{AppConfig, ShardConfig};
use common::util::common_utils::hash_index;
use common::{GroupId, UserId};
use once_cell::sync::OnceCell;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::transport::Channel;
use twox_hash::XxHash64;

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
            online_map: HashShardMap::new(GROUP_SHARD_SIZE, MEMBER_SHARD_SIZE),
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

impl ShardManager {
    pub fn new(shard_config: ShardConfig) -> Self {
        let shard_info = shard_config.clone();
        let mut info = ShardInfo::default();
        info.state = ShardState::Registered;
        let manager = Self {
            snapshot: ArcSwap::new(Arc::new(MemData::new())),
            shard_config: shard_info,
            current: ArcSwap::new(Arc::new(MemData::new())),
        };
        return manager;
    }
    pub fn hash_group_id(&self, group_id: &str) -> usize {
        use std::hash::{Hash, Hasher};
        use twox_hash::XxHash64;

        let mut hasher = XxHash64::with_seed(0);
        group_id.hash(&mut hasher);
        (hasher.finish() as usize) % GROUP_SHARD_SIZE
    }

    /// 计算群组成员索引（用于 group 成员缓存定位）
    pub fn hash_group_member_id(&self, group_id: &str, user_id: &str) -> usize {
        let mut hasher = XxHash64::with_seed(0);
        group_id.hash(&mut hasher);
        user_id.hash(&mut hasher);
        (hasher.finish() as usize) % MEMBER_SHARD_SIZE
    }
    pub fn get_node_addr(&self) -> &str {
        self.shard_config.shard_address.as_deref().expect("shard_address must be set")
    }
    pub async fn init_grpc_clients(
        &self,
        endpoints: Vec<String>,
    ) -> std::result::Result<HashMap<i32, ArbGroupServiceClient<Channel>>, Box<dyn std::error::Error>> {
        let mut clients = HashMap::new();
        let size = endpoints.len();
        for endpoint in endpoints {
            //跳过自动节点
            if endpoint == self.shard_config.shard_address.clone().unwrap() {
                continue;
            }
            let channel = Channel::from_shared(format!("http://{}", endpoint))?.connect().await?;
            let client = ArbGroupServiceClient::new(channel);
            clients.insert(hash_index(&endpoint, size as i32), client);
        }
        Ok(clients)
    }
    pub fn clear_current(&self) {
        self.current.store(Arc::new(MemData {
            shard_map: HashShardMap::new(GROUP_SHARD_SIZE, MEMBER_SHARD_SIZE),
            shard_info: RwLock::new(ShardInfo::default()),
        }));
    }

    pub fn clone_current_to_snapshot(&self) {
        let current = self.current.load();
        self.snapshot.store(current.clone());
    }
    pub fn clean_snapshot(&self) {
        self.snapshot.store(Arc::new(MemData {
            shard_map: HashShardMap::new(GROUP_SHARD_SIZE, MEMBER_SHARD_SIZE),
            shard_info: RwLock::new(ShardInfo::default()),
        }));
    }

    pub async fn init() {
        let app_cfg = AppConfig::get();
        let instance = Self::new(app_cfg.shard.clone().unwrap());
        instance.load_from().await;
        INSTANCE_COUNTRY.set(Arc::new(instance)).expect("INSTANCE already initialized");
    }

    /// 获取单例
    pub fn get() -> Arc<Self> {
        INSTANCE_COUNTRY.get().expect("INSTANCE is not initialized").clone()
    }
}
static INSTANCE_COUNTRY: OnceCell<Arc<ShardManager>> = OnceCell::new();

#[async_trait]
pub trait ShardManagerOpt: Send + Sync {
    async fn load_from(&self) -> anyhow::Result<()>;
    /// 添加用户到指定群组（自动根据 group_id 映射分片）
    fn add_member(&self, group_id: &GroupId, uid: &UserId) -> anyhow::Result<()>;
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

    /// 获取在线管理员
    async fn get_admin_member(&self, group_id: &GroupId) -> Result<Option<Vec<UserId>>>;
}

// pub trait GroupManagerOpt {
//     fn add_member(&self, group_id: &str, item: &MemberRef) -> Result<()>;
//     fn add_members(&self, group_id: &str, item_list: &Vec<MemberRef>) -> Result<()>;
//     fn remove_member(&self, group_id: &str, uid: &UserId) -> Result<()>;
//     fn remove_members(&self, group_id: &str, uid_list: &Vec<UserId>) -> Result<()>;
//     fn list_members_page(&self, group_id: &str, page: usize, page_size: usize) -> Result<Vec<String>, String>;
//     fn get_member_count(&self, group_id: &str) -> Result<usize>;
//
// }
