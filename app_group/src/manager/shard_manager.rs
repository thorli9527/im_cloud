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
use twox_hash::XxHash64;

pub const GROUP_SHARD_SIZE: usize = 16;
pub const MEMBER_SHARD_SIZE: usize = 8;
pub struct GroupMembersPage {
    pub total: usize,
    pub members: Vec<UserId>,
}

#[derive(Debug, Default)]
pub struct ShardInfo {
    pub version: u64,
    pub state: ShardState,
    pub last_update_time: u64,
    pub last_heartbeat: u64,
    pub total: i32,
    pub index: i32,
}
#[derive(Debug)]
pub struct MemData {
    pub group_shard_map: DashMap<String, DashMap<GroupId, ()>>,
    pub group_member_map: DashMap<String, DashMap<GroupId, Vec<DashSet<UserId>>>>, // ⬅️ 每个 group 成员可分片
    pub group_online_member_map: DashMap<String, DashMap<GroupId, DashSet<UserId>>>,
    pub shard_info: RwLock<ShardInfo>,
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
            current: ArcSwap::new(Arc::new(MemData {
                group_shard_map: Default::default(),
                group_member_map: Default::default(),
                group_online_member_map: Default::default(),
                shard_info: RwLock::new(ShardInfo::default()),
            })),
        }
    }
    pub fn get_node_addr(&self) -> &str {
        self.shard_config
            .shard_address
            .as_deref()
            .expect("shard_address must be set")
    }

    /// 计算群组分片索引（用于分配 group → shard）
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

    /// 添加群组至本地分片
    pub async fn add_group(&self, group_id: &GroupId) {
        // Step 1: 计算分片索引
        let shard_index = self.hash_group_id(group_id) as i32;
        let shard_key = format!("shard_{}", shard_index);

        // Step 2: 获取当前分片快照（Arc）
        let current = self.current.load();

        // Step 3: 检查分片状态
        let shard_info = current.shard_info.read().await;
        if shard_info.state != ShardState::Normal {
            log::warn!(
                "❌ 无法添加群组 group_id={}，当前分片状态为 {:?}，非 Normal 状态",
                group_id,
                shard_info.state
            );
            return;
        }
        drop(shard_info); // 提前释放锁

        // Step 4: 插入 group → shard 映射
        let group_map = current
            .group_shard_map
            .entry(shard_key.clone())
            .or_insert_with(DashMap::new);

        if group_map.contains_key(group_id) {
            log::info!("⚠️ 群组已存在: group_id={} 分片={}", group_id, shard_index);
            return;
        }
        group_map.insert(group_id.clone(), ());

        // Step 5: 初始化成员集合（支持成员分片）
        let member_map = current
            .group_member_map
            .entry(shard_key.clone())
            .or_insert_with(DashMap::new);

        member_map.entry(group_id.clone()).or_insert_with(|| {
            let mut shards = Vec::with_capacity(MEMBER_SHARD_SIZE);
            for _ in 0..MEMBER_SHARD_SIZE {
                shards.push(DashSet::new());
            }
            shards
        });

        // Step 6: 初始化在线成员集合
        current
            .group_online_member_map
            .entry(shard_key)
            .or_insert_with(DashMap::new)
            .entry(group_id.clone())
            .or_insert_with(DashSet::new);

        log::info!(
            "✅ 成功添加群组: group_id={} → 分片 {}",
            group_id,
            shard_index
        );
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
        log::info!(
            "❌ 群组删除成功: group_id={} 分片={}",
            group_id,
            shard_index
        );
    }

    /// 添加用户到指定群组（自动根据 group_id 映射分片）
    pub fn add_user_to_group(&self, group_id: &GroupId, user_id: &UserId) {
        let shard_index = self.hash_group_id(group_id) as i32;
        let member_index = self.hash_group_member_id(group_id, user_id);
        let shard_key = format!("shard_{}", shard_index);

        let current = self.current.load();

        let group_map = current
            .group_member_map
            .entry(shard_key.clone())
            .or_insert_with(DashMap::new);

        // 每个群组维护 N 个成员集合
        let member_shards = group_map.entry(group_id.clone()).or_insert_with(|| {
            let mut shards = Vec::with_capacity(MEMBER_SHARD_SIZE);
            for _ in 0..MEMBER_SHARD_SIZE {
                shards.push(DashSet::new());
            }
            shards
        });

        member_shards[member_index].insert(user_id.clone());

        log::debug!(
            "👤 用户 {} 添加至群 {} 分片={} 成员槽={}",
            user_id,
            group_id,
            shard_index,
            member_index
        );
    }
    /// 从指定群组中移除某个用户（自动计算分片）
    pub fn remove_user_from_group(&self, group_id: &GroupId, user_id: &UserId) {
        let shard_index = self.hash_group_id(group_id) as i32;
        let member_index = self.hash_group_member_id(group_id, user_id);
        let shard_key = format!("shard_{}", shard_index);

        let current = self.current.load();

        if let Some(group_map) = current.group_member_map.get(&shard_key) {
            if let Some(member_shards) = group_map.get(group_id) {
                if member_index >= member_shards.len() {
                    log::warn!(
                        "❌ 移除失败: 成员槽索引越界 group_id={} index={}",
                        group_id,
                        member_index
                    );
                    return;
                }

                // 从成员槽中移除用户
                if member_shards[member_index].remove(user_id).is_some() {
                    log::debug!(
                        "👤 用户 {} 从群组 {} 成员槽 {} 移除（分片 {}）",
                        user_id,
                        group_id,
                        member_index,
                        shard_index
                    );
                }

                // 如果该群组所有成员槽都为空，则清除该群组
                let group_empty = member_shards.iter().all(|slot| slot.is_empty());
                if group_empty {
                    group_map.remove(group_id);
                    log::debug!("⚠️ 群组 {} 无成员，已移除", group_id);
                }

                // 如果该分片已无任何群组，清除该 shard
                if group_map.is_empty() {
                    current.group_member_map.remove(&shard_key);
                    log::debug!("⚠️ 分片 {} 无群组缓存，已移除", shard_key);
                }
            }
        }
    }

    /// 获取某个群组的所有成员 ID 列表
    pub fn get_users_for_group(&self, group_id: &GroupId) -> Option<Vec<UserId>> {
        let shard_index = self.hash_group_id(group_id) as i32;
        let shard_key = format!("shard_{}", shard_index);

        let current = self.current.load();

        // 提前 clone 出用户集合
        if let Some(group_map) = current.group_member_map.get(&shard_key) {
            if let Some(member_shards) = group_map.get(group_id) {
                let users = member_shards
                    .iter()
                    .flat_map(|shard| shard.iter().map(|u| u.key().clone()))
                    .collect::<Vec<UserId>>();
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

        let current = self.current.load();

        if let Some(group_map) = current.group_member_map.get(&shard_key) {
            if let Some(member_shards) = group_map.get(group_id) {
                let all_users: Vec<UserId> = member_shards
                    .iter()
                    .flat_map(|shard| shard.iter().map(|u| u.key().clone()))
                    .skip(offset)
                    .take(limit)
                    .collect();
                return all_users;
            }
        }

        vec![]
    }
    pub fn get_group_member_total_count(&self, group_id: &str) -> Option<usize> {
        let shard_index = self.hash_group_id(group_id) as i32;
        let shard_key = format!("shard_{}", shard_index);

        self.current
            .load()
            .group_member_map
            .get(&shard_key)
            .and_then(|group_map| {
                group_map.get(group_id).map(|member_shards| {
                    member_shards
                        .iter()
                        .map(|shard| shard.len())
                        .sum::<usize>()
                })
            })
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
                    self.current
                        .load()
                        .group_online_member_map
                        .remove(&shard_key);
                }
            }
        }
    }
    pub fn clear_current(&self) {
        self.current.store(Arc::new(MemData {
            group_shard_map: Default::default(),
            group_member_map: Default::default(),
            group_online_member_map: Default::default(),
            shard_info: RwLock::new(ShardInfo::default()),
        }));
    }

    pub fn clone_current_to_snapshot(&self) {
        let current = self.current.load();
        self.snapshot.store(current.clone());
    }
    pub fn clean_snapshot(&self) {
        self.snapshot.store(Arc::new(MemData {
            group_shard_map: Default::default(),
            group_member_map: Default::default(),
            group_online_member_map: Default::default(),
            shard_info: RwLock::new(ShardInfo::default()),
        }));
    }

    pub fn init() {
        let app_cfg = AppConfig::get();
        let instance = Self::new(app_cfg.shard.clone().unwrap());
        INSTANCE_COUNTRY
            .set(Arc::new(instance))
            .expect("INSTANCE already initialized");
    }

    /// 获取单例
    pub fn get() -> Arc<Self> {
        INSTANCE_COUNTRY
            .get()
            .expect("INSTANCE is not initialized")
            .clone()
    }
}
static INSTANCE_COUNTRY: OnceCell<Arc<ShardManager>> = OnceCell::new();
