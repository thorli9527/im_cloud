use dashmap::mapref::multiple::RefMulti;
use dashmap::{DashMap, DashSet};
// 高性能并发哈希表
use deadpool_redis::{
    redis::AsyncCommands,
    Pool,
};
use once_cell::sync::OnceCell;
use redis::streams::{StreamReadOptions, StreamReadReply};
use redis::{cmd, from_redis_value, RedisResult};
// === Imports ===
// 引入所需标准库、第三方库和项目内模块
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
// === Imports ===

use serde::{Deserialize, Serialize};
use tokio::sync::Notify;
use tokio::time::sleep;

use common::errors::AppError;

// === Constants ===
const USER_ONLINE_TTL_SECS: u64 = 30;
const STREAM_KEY: &str = "user:events";
const CONSUMER_GROUP: &str = "user_events_group";
const CONSUMER_NAME: &str = "user_manager";
const SHARD_COUNT: usize = 8;
const MAX_CLEAN_COUNT: usize = 100;

// === 设备类型 ===
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum DeviceType {
    Unknown = 0,
    Mobile = 1,
    Desktop = 2,
    Web = 3,
}

impl From<u8> for DeviceType {
    fn from(value: u8) -> Self {
        match value {
            1 => DeviceType::Mobile,
            2 => DeviceType::Desktop,
            3 => DeviceType::Web,
            _ => DeviceType::Unknown,
        }
    }
}


// === 用户事件枚举 ===
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum UserEvent {
    GroupLeave { group_id: String, user_id: String },
    GroupJoin { group_id: String, user_id: String },
    Online { user_id: String, device: DeviceType },
    Offline { user_id: String },
}

// === 分片群组结构 ===
#[derive(Debug, Clone)]
struct ShardedGroupMap {
    shards: Arc<Vec<DashMap<String, DashSet<String>>>>,
}

impl ShardedGroupMap {
    pub fn new() -> Self {
        let shards = Arc::new((0..SHARD_COUNT).map(|_| DashMap::new()).collect());
        Self { shards }
    }
    fn hash(key: &str) -> usize {
        fxhash::hash32(key.as_bytes()) as usize % SHARD_COUNT
    }
    pub fn get(&self, key: &str) -> Option<dashmap::mapref::one::Ref<String, DashSet<String>>> {
        self.shards[Self::hash(key)].get(key)
    }
    pub fn get_mut(&self, key: &str) -> Option<dashmap::mapref::one::RefMut<String, DashSet<String>>> {
        self.shards[Self::hash(key)].get_mut(key)
    }
    pub fn entry(&self, key: String) -> dashmap::mapref::entry::Entry<String, DashSet<String>> {
        self.shards[Self::hash(&key)].entry(key)
    }
    pub fn remove(&self, key: &str) {
        self.shards[Self::hash(key)].remove(key);
    }
    pub fn iter(&self) -> impl Iterator<Item = RefMulti<String, DashSet<String>>> + '_ {
        self.shards.iter().flat_map(|shard| shard.iter())
    }
}

// === Redis 用户管理器 ===
#[derive(Clone, Debug)]
pub struct RedisUserManager {
    redis_pool: Pool,
    local_online_shards: Arc<Vec<DashMap<String, ()>>>,
    local_group_map: ShardedGroupMap,
    is_initialized: Arc<AtomicBool>,
    init_notify: Arc<Notify>,
    node_id: usize,
    node_total: usize,
    use_local_cache: bool,
}

impl RedisUserManager {
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
        
        if !use_local_cache{
            return manager
        }

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

        let cleaner = manager.clone();
        tokio::spawn(async move {
            loop {
                sleep(Duration::from_secs(300)).await;
                if let Err(e) = cleaner.clean_empty_groups().await {
                    eprintln!("[RedisUserManager] ❌ 清理空群组失败: {:?}", e);
                }
            }
        });

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
    pub async fn find_member(&self, group_id: &str, user_id: &str) -> Result<bool,AppError> {
        if self.use_local_cache{
            let local_has=self.local_group_map.get(group_id)
                .map(|set| set.contains(user_id))
                .unwrap_or(false);
            if local_has{
                return Ok(local_has);
            }
        }
        let mut conn = self.redis_pool.get().await?;
        let string = format!("group:{}", group_id);
        Ok(conn.sismember(string, user_id).await?)
    }

    // 设置用户上线（带事件广播）
    /// 设置用户上线（带事件广播，含设备类型）
    pub async fn online(&self, user_id: &str, device: DeviceType) -> Result<(), AppError> {
        if self.use_local_cache && self.is_responsible(user_id) {
            self.get_online_shard(user_id).insert(user_id.to_string(), ());
        }
        let mut conn = self.redis_pool.get().await?;
        let _: () = conn.set_ex(format!("online:user:{}", user_id), "1", USER_ONLINE_TTL_SECS).await?;
        let payload = serde_json::to_string(&UserEvent::Online {
            user_id: user_id.to_string(),
            device,
        })?;
        let _: () = conn.xadd(STREAM_KEY, "*", &[("payload", &payload)]).await?;
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
        if self.use_local_cache {
            self.local_group_map.entry(group_id.to_string()).or_insert_with(DashSet::new).insert(user_id.to_string());
        }
        let payload = serde_json::to_string(&UserEvent::GroupJoin { group_id: group_id.to_string(), user_id: user_id.to_string() })?;
        let _:()=conn.xadd(STREAM_KEY, "*", &[("payload", &payload)]).await?;
        Ok(())
    }

    /// 分页查询群组内成员列表
    pub async fn list_group_members(
        &self,
        group_id: &str,
        page: u64,
        size: u64,
    ) -> Result<Vec<String>, AppError> {
        // 否则从 Redis
        let mut conn = self.redis_pool.get().await?;
        let key = format!("group:{}", group_id);
        let all_members: Vec<String> = conn.smembers(&key).await.unwrap_or_default();
        let start = ((page - 1) * size) as usize;
        let end = (start + size as usize).min(all_members.len());

        if start >= all_members.len() {
            Ok(vec![])
        } else {
            Ok(all_members[start..end].to_vec())
        }
    }

    /// 分页查询用户加入的群组列表
    pub async fn list_user_groups(
        &self,
        user_id: &str,
        page: u64,
        size: u64,
    ) -> Result<Vec<String>, AppError> {
        let mut conn = self.redis_pool.get().await?;

        let key = format!("user:{}:groups", user_id);

        // 1. 拿到所有群组 ID
        let group_ids: Vec<String> = conn.smembers(&key).await.unwrap_or_default();

        if group_ids.is_empty() {
            return Ok(vec![]);
        }

        // 2. 做本地分页
        let start = ((page - 1) * size) as usize;
        let end = (start + size as usize).min(group_ids.len());

        if start >= group_ids.len() {
            Ok(vec![])
        } else {
            Ok(group_ids[start..end].to_vec())
        }
    }

    /// 解散群组（删除成员并广播事件）
    pub async fn dismiss_group(&self, group_id: &str) -> Result<(), AppError> {
        let mut conn = self.redis_pool.get().await?;

        // 获取成员列表（从 Redis）
        let members: Vec<String> = conn.smembers(format!("group:{}", group_id)).await.unwrap_or_default();

        // 删除 Redis 中的群组 key
        let _: () = conn.del(format!("group:{}", group_id)).await?;
        if self.use_local_cache{
            // 清理本地缓存
            self.local_group_map.remove(group_id);
        }
        // 向每个成员广播 GroupLeave 事件（可选但推荐）
        for user_id in &members {
            let event = UserEvent::GroupLeave {
                group_id: group_id.to_string(),
                user_id: user_id.to_string(),
            };
            let payload = serde_json::to_string(&event)?;
            let _: () = conn.xadd(STREAM_KEY, "*", &[("payload", &payload)]).await?;
        }

        println!("[RedisUserManager] 🧨 解散群组 {}，成员数: {}", group_id, members.len());

        // 可选：写日志记录
        // self.add_log(group_id, operator_user, None, GroupOperationType::Dismiss).await?;

        Ok(())
    }

    // 从群组中移除用户
    pub async fn remove_from_group(&self, group_id: &str, user_id: &str) -> Result<(), AppError> {
        let mut conn = self.redis_pool.get().await?;
        let _:()=conn.srem(format!("group:{}", group_id), user_id).await?;
        if self.use_local_cache {
            if let Some(mut group) = self.local_group_map.get_mut(group_id) {
                group.remove(user_id);
            }
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

    // 启动 Redis Stream 消费器，监听用户上下线事件
    /// 启动 Redis Stream 消费器，监听用户上下线和群组变动事件
    /// 启动 Redis Stream 消费器，监听用户上下线和群组事件
    pub async fn start_stream_event_consumer(&self) -> Result<(), AppError> {
        let pool = self.redis_pool.clone();
        let shards = self.local_online_shards.clone();
        let groups = self.local_group_map.clone();
        let node_id = self.node_id;
        let node_total = self.node_total;

        tokio::spawn(async move {
            // 初始化消费者组（幂等）
            if let Ok(mut conn) = pool.get().await {
                let _ = cmd("XGROUP")
                    .arg("CREATE")
                    .arg(STREAM_KEY)
                    .arg(CONSUMER_GROUP)
                    .arg("0")
                    .arg("MKSTREAM")
                    .query_async::<()>(&mut conn)
                    .await
                    .or_else(|e| {
                        if e.to_string().contains("BUSYGROUP") {
                            Ok(())
                        } else {
                            Err(e)
                        }
                    });
            }

            loop {
                if let Ok(mut conn) = pool.get().await {
                    let opts = StreamReadOptions::default()
                        .group(CONSUMER_GROUP, CONSUMER_NAME)
                        .count(10)
                        .block(5000);

                    let result: RedisResult<StreamReadReply> =
                        conn.xread_options::<_, _, StreamReadReply>(&[STREAM_KEY], &[">"], &opts).await;

                    if let Ok(reply) = result {
                        for stream in reply.keys {
                            for entry in stream.ids {
                                if let Some(payload_value) = entry.map.get("payload") {
                                    // 解析 Redis value 为 String
                                    let payload_str: String = match from_redis_value(payload_value) {
                                        Ok(val) => val,
                                        Err(e) => {
                                            eprintln!("[RedisUserManager] ❌ payload 类型错误: {:?}", e);
                                            continue;
                                        }
                                    };

                                    // 解析 JSON -> UserEvent
                                    match serde_json::from_str::<UserEvent>(&payload_str) {
                                        Ok(event) => {
                                            handle_user_event(&event, &shards, &groups, node_id, node_total).await;

                                            // ACK 消息
                                            let _: RedisResult<()> = conn
                                                .xack(STREAM_KEY, CONSUMER_GROUP, &[&entry.id])
                                                .await;

                                        }
                                        Err(e) => {
                                            eprintln!(
                                                "[RedisUserManager] ❗️事件反序列化失败: {:?}, 内容: {}",
                                                e, payload_str
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // 防止空转 CPU 爆炸
                sleep(Duration::from_millis(200)).await;
            }
        });

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
async fn handle_user_event(
    event: &UserEvent,
    shards: &Vec<DashMap<String, ()>>,
    groups: &ShardedGroupMap,
    node_id: usize,
    node_total: usize,
) {
    match event {
        UserEvent::Online { user_id, device } => {
            if is_responsible(user_id, node_id, node_total) {
                let shard = &shards[hash_user(user_id) % SHARD_COUNT];
                shard.insert(user_id.clone(), ());
                println!("[UserOnline] user_id = {}, device = {:?}", user_id, device);
            }
        }
        UserEvent::Offline { user_id } => {
            if is_responsible(user_id, node_id, node_total) {
                let shard = &shards[hash_user(user_id) % SHARD_COUNT];
                shard.remove(user_id);
            }
        }
        UserEvent::GroupJoin { group_id, user_id } => {
            groups
                .entry(group_id.clone())
                .or_insert_with(DashSet::new)
                .insert(user_id.clone());
        }
        UserEvent::GroupLeave { group_id, user_id } => {
            if let Some(mut group) = groups.get_mut(group_id) {
                group.remove(user_id);
            }
        }
    }
}

#[inline]
fn hash_user(user_id: &str) -> usize {
    fxhash::hash32(user_id.as_bytes()) as usize
}

#[inline]
fn is_responsible(user_id: &str, node_id: usize, node_total: usize) -> bool {
    (hash_user(user_id) % node_total) == node_id
}