use crate::protocol::rpc_arb_models::ShardState;
use crate::protocol::rpc_arb_server::arb_server_rpc_service_client::ArbServerRpcServiceClient;
use anyhow::Result;
use common::config::{AppConfig, ShardConfig};
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;
use dashmap::{DashMap, DashSet};
use mongodb::Database;
use tokio::sync::RwLock;
use tonic::Status;
use tonic::codegen::http::status;
use tonic::transport::Channel;
use tracing::log;
use common::{GroupId, UserId};
use once_cell::sync::OnceCell;
const GROUP_SHARD_SIZE: usize = 16;
const MEMBER_SHARD_SIZE: usize = 8;
#[derive(Debug)]
pub struct ShardInfo {
    pub node_addr: String,
    pub version: u64,
    pub state: ShardState,
    pub last_update_time: u64,
    pub last_heartbeat: u64,
}
#[derive(Debug)]
pub struct ShardManager {
    //快速分片信息
    pub snapshot: Arc<RwLock<Option<ShardInfo>>>,
    //当前快片信息
    pub current: Arc<RwLock<ShardInfo>>,
    //分片配置
    pub shard_config: ShardConfig,
    //分片仲裁服务器接口信息
    pub arb_client: Option<ArbServerRpcServiceClient<Channel>>,
    //群组分片缓存
    pub group_shard_map: Arc<DashMap<String, DashMap<GroupId, ()>>>,
    //群组成员缓存
    pub group_member_map: Arc<DashMap<String, DashMap<GroupId, DashSet<UserId>>>>,
    //分片总数
    pub total: i32,
    //分片索引
    pub index: i32,
}

impl ShardManager {
    pub fn new(shard_config: ShardConfig) -> Self {
        let shard_info = shard_config.clone();
        let node_addr = shard_config
            .shard_address
            .clone()
            .expect("shard_address must be set");

        Self {
            snapshot: Arc::new(RwLock::new(None)),
            current: Arc::new(RwLock::new(ShardInfo {
                node_addr,
                version: 0,
                state: ShardState::Unspecified,
                last_update_time: 0,
                last_heartbeat: 0,
            })),
            arb_client: None,
            shard_config: shard_info,
            group_shard_map: Arc::new(DashMap::new()),
            group_member_map: Arc::new(DashMap::new()),
            index:0,
            total: 0,
        }
    }
    pub fn get_node_addr(&self) -> &str {
        self.shard_config
            .shard_address
            .as_deref()
            .expect("shard_address must be set")
    }
    async fn client_init(&mut self) -> Result<&mut ArbServerRpcServiceClient<Channel>> {
        if self.arb_client.is_none() {
            let server_host = self.shard_config.server_host.clone().unwrap();
            let client = ArbServerRpcServiceClient::connect(server_host).await?;
            self.arb_client = Some(client);
        }
        Ok(self.arb_client.as_mut().unwrap())
    }

    /// 计算群组分片索引（用于分配 group → shard）
    fn hash_group_id(&self, group_id: &str) -> usize {
        use twox_hash::XxHash64;
        use std::hash::{Hash, Hasher};

        let mut hasher = XxHash64::with_seed(0);
        group_id.hash(&mut hasher);
        (hasher.finish() as usize) % GROUP_SHARD_SIZE
    }

    /// 计算群组成员索引（用于 group 成员缓存定位）
    fn hash_group_member_id(&self, group_id: &str, user_id: &str) -> usize {
        let mut hasher = DefaultHasher::new();
        group_id.hash(&mut hasher);
        user_id.hash(&mut hasher);
        (hasher.finish() as usize) % MEMBER_SHARD_SIZE
    }

    /// 添加群组至本地分片
    pub fn add_group(&self, group_id: &GroupId) {
        // Step 1: 计算分片索引
        let shard_index = self.hash_group_id(&group_id) as i32;

        // Step 2: 构造当前 shard key
        let shard_key = format!("shard_{}", shard_index);

        // Step 3: 检查是否属于当前节点负责的分片（可选）
        if shard_index != self.index {
            // 非本分片群组，可选择忽略或打日志
            log::debug!("Group {} not assigned to local shard {}, skip", group_id, self.index);
            return;
        }

        // Step 4: 插入 group → shard 映射
        self.group_shard_map
            .entry(shard_key.clone())
            .or_insert_with(DashMap::new)
            .insert(group_id.clone(), ());

        // Step 5: 初始化成员缓存（空 set）
        self.group_member_map
            .entry(shard_key)
            .or_insert_with(DashMap::new)
            .entry(group_id.clone())
            .or_insert_with(DashSet::new);

        log::info!("✅ 添加群组成功: group_id={} 分片={}", group_id, shard_index);
    }

    /// 删除群组及其缓存信息（包括 group_shard_map 和 group_member_map 中的所有记录）
    pub fn remove_group(&self, group_id: GroupId) {
        // 1. 计算分片索引
        let shard_index = self.hash_group_id(&group_id) as i32;
        let shard_key = format!("shard_{}", shard_index);

        // 2. 从 group_shard_map 中移除
        if let Some(group_map) = self.group_shard_map.get(&shard_key) {
            group_map.remove(&group_id);
            if group_map.is_empty() {
                self.group_shard_map.remove(&shard_key);
            }
        }
        // 3. 从 group_member_map 中移除该群组的成员缓存
        if let Some(member_map) = self.group_member_map.get(&shard_key) {
            member_map.remove(&group_id);
            if member_map.is_empty() {
                self.group_member_map.remove(&shard_key);
            }
        }
        // 4. 打日志记录
        log::info!("❌ 群组删除成功: group_id={} 分片={}", group_id, shard_index);
    }

    /// 添加用户到指定群组（自动根据 group_id 映射分片）
    pub fn add_user_to_group(&self, group_id: GroupId, user_id: UserId) {
        // 1. 根据 group_id 映射到 shard_index 和 shard_key
        let shard_index = self.hash_group_id(&group_id) as i32;
        let shard_key = format!("shard_{}", shard_index);

        // 2. 插入成员
        let group_map = self
            .group_member_map
            .entry(shard_key.clone())
            .or_insert_with(DashMap::new);

        let user_set = group_map
            .entry(group_id.clone())
            .or_insert_with(DashSet::new);

        user_set.insert(user_id.clone());

        log::debug!("👤 用户 {} 添加至群 {}（分片 {}）", user_id, group_id, shard_index);
    }
    /// 从指定群组中移除某个用户（自动计算分片）
    pub fn remove_user_from_group(&self, group_id: GroupId, user_id: UserId) {
        // 1. 获取分片索引和 key
        let shard_index = self.hash_group_id(&group_id) as i32;
        let shard_key = format!("shard_{}", shard_index);

        // 2. 获取对应群组成员集合
        if let Some(group_map) = self.group_member_map.get(&shard_key) {
            if let Some(user_set) = group_map.get(&group_id) {
                user_set.remove(&user_id);

                // 3. 若该群组成员已空，清除群组记录
                if user_set.is_empty() {
                    group_map.remove(&group_id);
                    log::debug!("群组 {} 成员已清空，移除 group", group_id);
                }

                // 4. 若该分片下无群组，清除整个 shard entry
                if group_map.is_empty() {
                    self.group_member_map.remove(&shard_key);
                    log::debug!("分片 {} 无群组缓存，移除 shard", shard_key);
                }

                log::debug!("👤 用户 {} 移除自群组 {}（分片 {}）", user_id, group_id, shard_index);
            }
        }
    }
    /// 获取某个群组的所有成员 ID 列表
    pub fn get_users_for_group(&self, group_id: GroupId) -> Option<Vec<UserId>> {
        let shard_index = self.hash_group_id(&group_id) as i32;
        let shard_key = format!("shard_{}", shard_index);

        // 尝试获取群组成员集合并 clone 出用户 ID
        if let Some(group_map) = self.group_member_map.get(&shard_key) {
            if let Some(user_set) = group_map.get(&group_id) {
                let users: Vec<UserId> = user_set.iter().map(|u| u.key().clone()).collect();
                return Some(users);
            }
        }
        None
    }

    /// 获取群组成员分页列表
    pub fn get_group_members_page(
        &self,
        group_id: &str,
        offset: usize,
        limit: usize,
    ) -> Vec<UserId> {
        let shard_index = self.hash_group_id(group_id) as i32;
        let shard_key = format!("shard_{}", shard_index);

        if let Some(group_map) = self.group_member_map.get(&shard_key) {
            if let Some(user_set) = group_map.get(group_id) {
                // 提前 collect 避免借用生命周期问题
                let all_users: Vec<UserId> = user_set.iter().map(|u| u.key().clone()).collect();
                return all_users
                    .into_iter()
                    .skip(offset)
                    .take(limit)
                    .collect();
            }
        }
        // 默认返回空
        vec![]
    }
    pub fn init() {
        let app_cfg = AppConfig::get();
        let instance = Self::new(app_cfg.shard.clone().unwrap());
        INSTANCE_COUNTRY.set(Arc::new(instance)).expect("INSTANCE already initialized");
    }

    /// 获取单例
    pub fn get() -> Arc<Self> {
        INSTANCE_COUNTRY.get().expect("INSTANCE is not initialized").clone()
    }
}
static INSTANCE_COUNTRY: OnceCell<Arc<ShardManager>> = OnceCell::new();
#[async_trait]
/// 分片管理器核心操作定义接口，适用于支持动态迁移、健康上报、状态切换的分布式群组服务。
pub trait ShardManagerOpt: Send + Sync {
    /// 初始化管理器（例如加载缓存、连接仲裁器、预拉取分片信息等）
    async fn init(&mut self) -> Result<()>;

    /// 注册当前节点到仲裁中心或注册服务，用于初始接入和负载调度识别
    async fn register_node(&mut self) -> Result<()>;
    /// 设置某群组迁移状态为“准备中”
    /// 表示目标节点已准备好接收群组（例如缓存准备、校验完成等）
    async fn change_preparing(&mut self) -> Result<()>;
    /// 将群组分片状态设置为“迁移中”
    /// 通常意味着不再接受新写入，同时准备数据转移
    async fn change_migrating(&mut self) -> Result<()>;

    /// 同步当前群组列表（通常从仲裁服务或中心节点拉取最新群组分配情况）
    async fn sync_groups(&mut self) -> Result<()>;

    /// 同步群组成员列表信息（确保迁移前/后成员视图一致）
    async fn sync_group_members(&mut self) -> Result<()>;

    /// 设置群组状态为“迁移失败”
    /// 可用于回滚操作或触发异常迁移重试逻辑
    async fn change_failed(&mut self) -> Result<()>;

    /// 设置为“就绪”状态，表示目标节点已接管数据并可激活群组
    async fn change_ready(&mut self) -> Result<()>;

    /// 设置为“正常”状态，表示群组已完成迁移并稳定运行
    async fn change_normal(&mut self) -> Result<()>;
    ///准备下线
    async fn change_preparing_offline(&mut self) -> Result<()>;
    /// 节点下线
    async fn change_offline(&mut self) -> Result<()>;
    /// 向仲裁服务上报心跳信息（包括负载、分片列表等），用于节点健康检查
    async fn heartbeat(&mut self) -> Result<()>;
}
include!("shard_manager_impl.rs");
