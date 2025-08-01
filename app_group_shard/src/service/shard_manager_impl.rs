use crate::db::hash_shard_map::HashShardMap;
use crate::service::shard_manager::{MemData, ShardInfo, ShardManager, ShardManagerOpt, GROUP_SHARD_SIZE, MEMBER_SHARD_SIZE};
use arc_swap::ArcSwap;
use common::config::{AppConfig, ShardConfig};
use common::util::common_utils::hash_index;
use once_cell::sync::OnceCell;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::transport::Channel;
use twox_hash::XxHash64;
use biz_service::protocol::rpc::arb_group::arb_group_service_client::ArbGroupServiceClient;
use biz_service::protocol::rpc::arb_models::ShardState;

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
        instance.load_from_data().await.expect("load_from redis error");
        INSTANCE_COUNTRY.set(Arc::new(instance)).expect("INSTANCE already initialized");
    }

    /// 获取单例
    pub fn get() -> Arc<Self> {
        INSTANCE_COUNTRY.get().expect("INSTANCE is not initialized").clone()
    }
}
static INSTANCE_COUNTRY: OnceCell<Arc<ShardManager>> = OnceCell::new();
