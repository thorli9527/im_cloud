use crate::protocol::rpc_arb_models::ShardState;
use crate::protocol::rpc_arb_server::arb_server_rpc_service_client::ArbServerRpcServiceClient;
use anyhow::Result;
use arc_swap::ArcSwap;
use common::config::{AppConfig, ShardConfig};
use common::{GroupId, UserId};
use dashmap::{DashMap, DashSet};
use mongodb::Database;
use once_cell::sync::OnceCell;
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::codegen::http::status;
use tonic::transport::Channel;
use tonic::Status;
use tracing::log;
const GROUP_SHARD_SIZE: usize = 16;
const MEMBER_SHARD_SIZE: usize = 8;
#[derive(Debug,Default)]
pub struct ShardInfo {
    pub version: u64,
    pub state: ShardState,
    pub last_update_time: u64,
    pub last_heartbeat: u64,
    pub total: i32,
    pub index: i32,
}
#[derive(Debug)]
pub struct MemData{
    //群组分片缓存
    pub group_shard_map: DashMap<String, DashMap<GroupId, ()>>,
    //群组成员缓存
    pub group_member_map: DashMap<String, DashMap<GroupId, DashSet<UserId>>>,
    //群组成员在线缓存
    pub group_online_member_map: DashMap<String, DashMap<GroupId, DashSet<UserId>>>,
    //分片信息
    pub shard_info: RwLock<ShardInfo>

}

#[derive(Debug)]
pub struct ShardManager {
    //快速分片信息
    pub snapshot: ArcSwap<MemData>,
    //当前快片信息
    // pub current: Arc<RwLock<ShardInfo>>,
    //分片配置
    pub shard_config: ShardConfig,
    // 当前分片数据
    pub current: ArcSwap<MemData>,

}

impl ShardManager {
    pub fn new(shard_config: ShardConfig) -> Self {
        let shard_info = shard_config.clone();
        Self {
            snapshot: ArcSwap::new(Arc::new(MemData {
                group_shard_map: Default::default(),
                group_member_map: Default::default(),
                group_online_member_map: Default::default(),
                shard_info: RwLock::new(ShardInfo::default()),
            })),
            shard_config: shard_info,
            current:ArcSwap::new(Arc::new(MemData {
                group_shard_map: Default::default(),
                group_member_map: Default::default(),
                group_online_member_map: Default::default(),
                shard_info: RwLock::new(ShardInfo::default()),
            })),
            // group_shard_map: Arc::new(DashMap::new()),
            // group_member_map: Arc::new(DashMap::new()),
            // group_online_member_map: Arc::new(DashMap::new()),
        }
    }
    pub fn get_node_addr(&self) -> &str {
        self.shard_config
            .shard_address
            .as_deref()
            .expect("shard_address must be set")
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
    pub async fn add_group(&self, group_id: &GroupId) {
        // Step 1: 计算分片索引
        let shard_index = self.hash_group_id(&group_id) as i32;

        // Step 2: 构造当前 shard key
        let shard_key = format!("shard_{}", shard_index);

        // Step 3: 读取 shard_info，并判断是否存在
        let guard1 = self.current.load();
        let guard = guard1.shard_info.read().await;
        if guard.state != ShardState::Normal{
            log::error!("当前分片状态异常，请稍后再试");
            return;
        }

        // Step 4: 插入 group → shard 映射
        guard1
            .group_shard_map
            .entry(shard_key.clone())
            .or_insert_with(DashMap::new)
            .insert(group_id.clone(), ());

        // Step 5: 初始化成员缓存（空 set）
        guard1
            .group_member_map
            .entry(shard_key)
            .or_insert_with(DashMap::new)
            .entry(group_id.clone())
            .or_insert_with(DashSet::new);

        log::info!("✅ 添加群组成功: group_id={} 分片={}", group_id, shard_index);
    }

    /// 删除群组及其缓存信息（包括 group_shard_map 和 group_member_map 中的所有记录）
    pub fn remove_group(&self, group_id: &GroupId) {
        // 1. 计算分片索引
        let shard_index = self.hash_group_id(&group_id) as i32;
        let shard_key = format!("shard_{}", shard_index);

        // 2. 从 group_shard_map 中移除
        if let Some(group_map) = self.current.load().group_shard_map.get(&shard_key) {
            group_map.remove(group_id);
            if group_map.is_empty() {
                self.current.load().group_shard_map.remove(&shard_key);
            }
        }
        // 3. 从 group_member_map 中移除该群组的成员缓存
        if let Some(member_map) = self.current.load().group_member_map.get(&shard_key) {
            member_map.remove(group_id);
            if member_map.is_empty() {
                self.current.load().group_member_map.remove(&shard_key);
            }
        }
        // 4. 打日志记录
        log::info!("❌ 群组删除成功: group_id={} 分片={}", group_id, shard_index);
    }

    /// 添加用户到指定群组（自动根据 group_id 映射分片）
    pub fn add_user_to_group(&self, group_id: &GroupId, user_id: &UserId) {
        // 1. 根据 group_id 映射到 shard_index 和 shard_key
        let shard_index = self.hash_group_id(&group_id) as i32;
        let shard_key = format!("shard_{}", shard_index);

        // 2. 插入成员
        let guard = self.current.load();
        let group_map = guard
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
    pub fn remove_user_from_group(&self, group_id: &GroupId, user_id: &UserId) {
        // 1. 获取分片索引和 key
        let shard_index = self.hash_group_id(&group_id) as i32;
        let shard_key = format!("shard_{}", shard_index);

        // 2. 获取对应群组成员集合
        if let Some(group_map) = self.current.load().group_member_map.get(&shard_key) {
            if let Some(user_set) = group_map.get(group_id) {
                user_set.remove(user_id);

                // 3. 若该群组成员已空，清除群组记录
                if user_set.is_empty() {
                    group_map.remove(group_id);
                    log::debug!("群组 {} 成员已清空，移除 group", group_id);
                }

                // 4. 若该分片下无群组，清除整个 shard entry
                if group_map.is_empty() {
                    self.current.load().group_member_map.remove(&shard_key);
                    log::debug!("分片 {} 无群组缓存，移除 shard", shard_key);
                }

                log::debug!("👤 用户 {} 移除自群组 {}（分片 {}）", user_id, group_id, shard_index);
            }
        }
    }
    /// 获取某个群组的所有成员 ID 列表
    pub fn get_users_for_group(&self, group_id: &GroupId) -> Option<Vec<UserId>> {
        let shard_index = self.hash_group_id(&group_id) as i32;
        let shard_key = format!("shard_{}", shard_index);

        // 尝试获取群组成员集合并 clone 出用户 ID
        if let Some(group_map) = self.current.load().group_member_map.get(&shard_key) {
            if let Some(user_set) = group_map.get(group_id) {
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

        if let Some(group_map) = self.current.load().group_member_map.get(&shard_key) {
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
    pub fn get_group_member_total_count(&self, group_id: &str) -> Option<i32> {
        self.current.load().group_member_map
            .get(group_id)
            .map(|set| set.len() as i32)
    }
    pub fn mark_user_online(&self, group_id: &GroupId, user_id: &UserId) {
        let shard_index = self.hash_group_id(&group_id) as i32;
        let shard_key = format!("shard_{}", shard_index);

        // 1. 插入在线用户 → 群组映射
        let guard = self.current.load();
        let group_map = guard
            .group_online_member_map
            .entry(shard_key.clone())
            .or_insert_with(DashMap::new);

        let user_set = group_map
            .entry(group_id.clone())
            .or_insert_with(DashSet::new);
        user_set.insert(user_id.clone());
    }

    pub fn get_online_users_for_group(&self, group_id: &GroupId) -> Vec<UserId> {
        let shard_index = self.hash_group_id(&group_id) as i32;
        let shard_key = format!("shard_{}", shard_index);

        if let Some(group_map) = self.current.load().group_online_member_map.get(&shard_key) {
            if let Some(user_set) = group_map.get(group_id) {
                return user_set.iter().map(|u| u.key().clone()).collect();
            }
        }
        vec![]
    }
    pub fn mark_user_offline(&self, group_id: &GroupId, user_id: &UserId) {
        let shard_index = self.hash_group_id(&group_id) as i32;
        let shard_key = format!("shard_{}", shard_index);

        if let Some(group_map) = self.current.load().group_online_member_map.get(&shard_key) {
            if let Some(user_set) = group_map.get(group_id) {
                user_set.remove(user_id);

                if user_set.is_empty() {
                    group_map.remove(group_id);
                }

                if group_map.is_empty() {
                    self.current.load().group_online_member_map.remove(&shard_key);
                }
            }
        }
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

