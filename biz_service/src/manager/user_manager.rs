// === Imports ===
// 引入所需标准库、第三方库和项目内模块
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use dashmap::{DashMap, DashSet}; // 高性能并发哈希表
use deadpool_redis::{
    redis::{from_redis_value, AsyncCommands, RedisResult},
    Pool,
};
use dashmap::mapref::multiple::RefMulti;
use mongodb::Database;
use once_cell::sync::OnceCell;
use redis::cmd;
use serde::{Deserialize, Serialize};
use tokio::sync::Notify;
use tokio::time::sleep;

use common::errors::AppError;
use crate::biz_service::client_service::ClientService;
use crate::manager::init;

// === Constants ===
const USER_ONLINE_TTL_SECS: u64 = 30;  // 用户在线 TTL
const STREAM_KEY: &str = "user:events";  // Redis Stream 的 key
const CONSUMER_GROUP: &str = "user_events_group"; // 消费者组名称
const CONSUMER_NAME: &str = "user_manager"; // 消费者实例名称
const SHARD_COUNT: usize = 8; // 本地缓存分片数量
const MAX_CLEAN_COUNT: usize = 100; // 清理时最多删除的空群组数

// === 用户事件枚举 ===
// 用于 Redis Stream 消息传递的事件模型
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum UserEvent {
    GroupLeave { group_id: String, user_id: String },
    GroupJoin { group_id: String, user_id: String },
    Online { user_id: String },
    Offline { user_id: String },
}

// === 分片群组结构 ===
#[derive(Debug, Clone)]
struct ShardedGroupMap {
    shards: Arc<Vec<DashMap<String, DashSet<String>>>>, // 每个群组是一个 DashSet
}

impl ShardedGroupMap {
    pub fn new() -> Self {
        let shards = Arc::new((0..SHARD_COUNT).map(|_| DashMap::new()).collect());
        Self { shards }
    }

    // 哈希函数，用于定位 shard
    fn hash(key: &str) -> usize {
        fxhash::hash32(key.as_bytes()) as usize % SHARD_COUNT
    }

    // 获取只读引用
    pub fn get(&self, key: &str) -> Option<dashmap::mapref::one::Ref<String, DashSet<String>>> {
        self.shards[Self::hash(key)].get(key)
    }

    // 获取可变引用
    pub fn get_mut(&self, key: &str) -> Option<dashmap::mapref::one::RefMut<String, DashSet<String>>> {
        self.shards[Self::hash(key)].get_mut(key)
    }

    // 获取或插入 entry
    pub fn entry(&self, key: String) -> dashmap::mapref::entry::Entry<String, DashSet<String>> {
        self.shards[Self::hash(&key)].entry(key)
    }

    // 删除群组缓存
    pub fn remove(&self, key: &str) {
        self.shards[Self::hash(key)].remove(key);
    }

    // 迭代所有缓存项
    pub fn iter(&self) -> impl Iterator<Item = RefMulti<String, DashSet<String>>> + '_ {
        self.shards.iter().flat_map(|shard| shard.iter())
    }
}

// === Redis 用户管理器 ===
#[derive(Clone, Debug)]
pub struct RedisUserManager {
    redis_pool: Pool, // Redis 连接池
    local_online_shards: Arc<Vec<DashMap<String, ()>>>, // 本地在线用户缓存分片
    local_group_map: ShardedGroupMap, // 本地群组成员缓存
    is_initialized: Arc<AtomicBool>, // 是否初始化完成
    init_notify: Arc<Notify>, // 初始化完成通知器
    node_id: usize, // 当前节点编号
    node_total: usize, // 总节点数（用于分片责任判定）
    use_local_cache: bool, // 是否使用本地缓存加速
}

impl RedisUserManager {
    // 初始化新实例，启动后台任务：加载缓存、事件监听、定时清理
    pub fn new(redis_pool: Pool, node_id: usize, node_total: usize, use_local_cache: bool) -> Self {
        let online_shards = (0..SHARD_COUNT).map(|_| DashMap::new()).collect();
        let manager = Self {
            redis_pool,
            local_online_shards: Arc::new(online_shards),
            local_group_map: ShardedGroupMap::new(),
            is_initialized: Arc::new(AtomicBool::new(false)),
            init_notify: Arc::new(Notify::new()),
            node_id,
            node_total,
            use_local_cache,
        };

        // 后台初始化任务
        let manager_clone = manager.clone();
        tokio::spawn(async move {
            if manager_clone.use_local_cache {
                if let Err(e) = manager_clone.initialize_from_redis().await {
                    eprintln!("[RedisUserManager] 初始化失败: {:?}", e);
                }
            }
            if let Err(e) = manager_clone.start_stream_event_consumer().await {
                eprintln!("[RedisUserManager] 消费器启动失败: {:?}", e);
            }
            manager_clone.is_initialized.store(true, Ordering::SeqCst);
            manager_clone.init_notify.notify_waiters();
            println!("[RedisUserManager] ✅ 初始化完成");
        });

        // 启动空群组清理器
        let cleaner = manager.clone();
        tokio::spawn(async move {
            loop {
                sleep(Duration::from_secs(300)).await;
                if let Err(e) = cleaner.clean_empty_groups().await {
                    eprintln!("[RedisUserManager] ❌ 清理空群组失败: {:?}", e);
                }
            }
        });

        // 注册全局单例
        manager.init(manager.clone());
        manager
    }

    // 获取用户分片
    fn get_online_shard(&self, user_id: &str) -> &DashMap<String, ()> {
        let hash = fxhash::hash32(user_id.as_bytes());
        &self.local_online_shards[(hash as usize) % SHARD_COUNT]
    }

    // 判定是否由当前节点负责管理该用户
    fn is_responsible(&self, user_id: &str) -> bool {
        let hash = fxhash::hash32(user_id.as_bytes()) as usize;
        (hash % self.node_total) == self.node_id
    }

    // 等待初始化完成（异步）
    pub async fn wait_until_ready(&self) {
        if !self.is_ready() {
            self.init_notify.notified().await;
        }
    }

    // 检查是否初始化完成
    pub fn is_ready(&self) -> bool {
        self.is_initialized.load(Ordering::SeqCst)
    }

    // 获取某个群组的所有成员
    pub fn get_group_members(&self, group_id: &str) -> Vec<String> {
        self.local_group_map.get(group_id)
            .map(|set| set.iter().map(|id| id.clone()).collect())
            .unwrap_or_default()
    }

    // 设置用户上线（带事件广播）
    pub async fn online(&self, user_id: &str) -> Result<(), AppError> {
        if self.use_local_cache && self.is_responsible(user_id) {
            self.get_online_shard(user_id).insert(user_id.to_string(), ());
        }
        let mut conn = self.redis_pool.get().await?;
        let _:()=conn.set_ex(format!("online:user:{}", user_id), "1", USER_ONLINE_TTL_SECS).await?;
        let payload = serde_json::to_string(&UserEvent::Online { user_id: user_id.to_string() })?;
        let _:()=conn.xadd(STREAM_KEY, "*", &[("payload", &payload)]).await?;
        Ok(())
    }

    // 设置用户下线
    pub async fn offline(&self, user_id: &str) -> Result<(), AppError> {
        if self.use_local_cache && self.is_responsible(user_id) {
            self.get_online_shard(user_id).remove(user_id);
        }
        let mut conn = self.redis_pool.get().await?;
        let _:()=conn.del(format!("online:user:{}", user_id)).await?;
        let payload = serde_json::to_string(&UserEvent::Offline { user_id: user_id.to_string() })?;
        let _:()=conn.xadd(STREAM_KEY, "*", &[("payload", &payload)]).await?;
        Ok(())
    }

    // 判断用户是否在线
    pub async fn is_online(&self, user_id: &str) -> Result<bool, AppError> {
        if self.use_local_cache && self.is_responsible(user_id) && self.get_online_shard(user_id).contains_key(user_id) {
            return Ok(true);
        }
        let mut conn = self.redis_pool.get().await?;
        Ok(conn.exists(format!("online:user:{}", user_id)).await?)
    }

    // 添加用户至群组并广播事件
    pub async fn add_to_group(&self, group_id: &str, user_id: &str) -> Result<(), AppError> {
        let mut conn = self.redis_pool.get().await?;
        let _:()=conn.sadd(format!("group:{}", group_id), user_id).await?;
        self.local_group_map.entry(group_id.to_string()).or_insert_with(DashSet::new).insert(user_id.to_string());
        let payload = serde_json::to_string(&UserEvent::GroupJoin { group_id: group_id.to_string(), user_id: user_id.to_string() })?;
        let _:()=conn.xadd(STREAM_KEY, "*", &[("payload", &payload)]).await?;
        Ok(())
    }

    // 从群组中移除用户
    pub async fn remove_from_group(&self, group_id: &str, user_id: &str) -> Result<(), AppError> {
        let mut conn = self.redis_pool.get().await?;
        let _:()=conn.srem(format!("group:{}", group_id), user_id).await?;
        if let Some(mut group) = self.local_group_map.get_mut(group_id) {
            group.remove(user_id);
        }
        let payload = serde_json::to_string(&UserEvent::GroupLeave { group_id: group_id.to_string(), user_id: user_id.to_string() })?;
        let _:()=conn.xadd(STREAM_KEY, "*", &[("payload", &payload)]).await?;
        Ok(())
    }

    // 清理空群组
    pub async fn clean_empty_groups(&self) -> Result<(), AppError> {
        let mut conn = self.redis_pool.get().await?;
        let mut to_delete = Vec::with_capacity(MAX_CLEAN_COUNT);
        let mut scanned = 0;

        for entry in self.local_group_map.iter() {
            if to_delete.len() >= MAX_CLEAN_COUNT { break; }
            scanned += 1;
            if entry.value().is_empty() {
                to_delete.push(entry.key().clone());
            }
        }

        for group_id in &to_delete {
            let _:()=conn.del(format!("group:{}", group_id)).await?;
            self.local_group_map.remove(group_id);
            println!("[RedisUserManager] 🧹 清理空群组: {}", group_id);
        }

        println!("[RedisUserManager] ✅ 清理完成: {} / {}", to_delete.len(), scanned);
        Ok(())
    }

    // 从 Redis 初始化本地在线状态和群组映射
    pub async fn initialize_from_redis(&self) -> Result<(), AppError> {
        let mut conn = self.redis_pool.get().await?;
        let mut cursor = 0u64;
        loop {
            let (next_cursor, keys): (u64, Vec<String>) = cmd("SCAN")
                .arg(cursor).arg("MATCH").arg("online:user:*").arg("COUNT").arg(100)
                .query_async(&mut conn).await?;
            for key in keys {
                if let Some(user_id) = key.strip_prefix("online:user:") {
                    if self.is_responsible(user_id) {
                        self.get_online_shard(user_id).insert(user_id.to_string(), ());
                    }
                }
            }
            if next_cursor == 0 { break; }
            cursor = next_cursor;
        }
        self.reload_group_map().await
    }

    // 重新加载群组映射
    pub async fn reload_group_map(&self) -> Result<(), AppError> {
        let mut conn = self.redis_pool.get().await?;
        let mut cursor = 0u64;
        loop {
            let (next_cursor, keys): (u64, Vec<String>) = cmd("SCAN")
                .arg(cursor).arg("MATCH").arg("group:*").arg("COUNT").arg(100)
                .query_async(&mut conn).await?;
            for key in &keys {
                if let Some(group_id) = key.strip_prefix("group:") {
                    let members: HashSet<String> = conn.smembers(&key).await.unwrap_or_default();
                    let group = self.local_group_map.entry(group_id.to_string()).or_insert_with(DashSet::new);
                    for user_id in members {
                        group.insert(user_id);
                    }
                }
            }
            if next_cursor == 0 { break; }
            cursor = next_cursor;
        }
        println!("[RedisUserManager] ✅ 群组映射已重新加载");
        Ok(())
    }

    // 启动 Stream 消费器（待补充逻辑）
    pub async fn start_stream_event_consumer(&self) -> Result<(), AppError> {
        Ok(())
    }

    // 注册为全局单例
    fn init(&self, instance: RedisUserManager) {
        INSTANCE.set(Arc::new(instance)).expect("INSTANCE already initialized");
    }

    // 获取全局实例
    pub fn get() -> Arc<Self> {
        INSTANCE.get().expect("INSTANCE is not initialized").clone()
    }
}

// 单例静态变量
static INSTANCE: OnceCell<Arc<RedisUserManager>> = OnceCell::new();
