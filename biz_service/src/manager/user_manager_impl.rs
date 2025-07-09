use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::time::Duration;
use deadpool_redis::redis::{cmd, AsyncCommands};
use common::config::AppConfig;
use common::util::common_utils::build_md5_with_key;
use crate::biz_service::agent_service::AgentService;
use crate::entitys::group_entity::GroupInfo;
use crate::entitys::group_member::GroupMemberMeta;
use crate::manager::common::SHARD_COUNT;
use crate::manager::local_group_manager::LocalGroupManagerOpt;
use once_cell::sync::OnceCell;
use tokio::time::sleep;
pub const MAX_CLEAN_COUNT: usize = 100;
pub const USER_ONLINE_TTL_SECS: u64 = 30;
pub const STREAM_KEY: &str = "user:events";
pub const CONSUMER_GROUP: &str = "user_events_group";
pub const CONSUMER_NAME: &str = "user_manager";
impl UserManager {
    /// 构造新的 UserManager 实例
    ///
    /// # 参数
    /// - `pool`: Redis 连接池
    /// - `node_id`: 当前节点编号
    /// - `node_total`: 节点总数
    /// - `shard_count`: 本地在线缓存分片数量
    /// - `use_local_cache`: 是否启用本地缓存
    /// - `group_map`: 预初始化的分片群组缓存结构
    pub fn new(pool: RedisPool, use_local_cache: bool) -> Self {
        let online_shards = (0..SHARD_COUNT).map(|_| DashMap::new()).collect();
        let local_group_manager = LocalGroupManager::get();
        let manager = Self {
            pool,
            local_online_shards: Arc::new(online_shards),
            local_group_manager,
            is_initialized: Arc::new(AtomicBool::new(false)),
            init_notify: Arc::new(Notify::new()),
            use_local_cache,
            friend_map: Arc::new(DashMap::<String, DashMap<UserId, ()>>::new()),
        };

        if !use_local_cache {
            manager.init(manager.clone());
            return manager;
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
                if let Err(e) = cleaner.clean().await {
                    eprintln!("[RedisUserManager] ❌ 清理空群组失败: {:?}", e);
                }
            }
        });
        manager.init(manager.clone());
        manager
    }
    /// 清理本地在线缓存（可选：按条件/全量）
    /// - 若启用本地缓存，则遍历每个分片中的用户，检查其是否仍在 Redis 中存在在线记录。
    /// - 若 Redis 无对应数据，则删除本地项。
    pub async fn clean_local_online_cache(&self) -> anyhow::Result<usize> {
        if !self.use_local_cache {
            return Ok(0); // 未启用缓存则跳过
        }

        let mut conn = self.pool.get().await?;
        let mut removed_count = 0;

        for shard in self.local_online_shards.iter() {
            let users: Vec<UserId> = shard.iter().map(|e| e.key().clone()).collect();
            for user_id in users {
                let redis_key_prefix = format!("online:user:agent:");
                let pattern = format!("{}*:{}:*", redis_key_prefix, user_id);

                // 检查 Redis 是否存在该用户在线记录（模糊匹配 agent_id + device_type）
                let exists: Vec<String> = cmd("KEYS")
                    .arg(&pattern)
                    .query_async(&mut conn)
                    .await
                    .unwrap_or_default();

                if exists.is_empty() {
                    shard.remove(&user_id);
                    removed_count += 1;
                    println!("[UserManager] 🧹 清理离线用户缓存: {}", user_id);
                }
            }
        }

        println!(
            "[UserManager] ✅ 在线缓存清理完成，总计 {} 个用户",
            removed_count
        );
        Ok(removed_count)
    }
    pub async fn initialize_from_redis(&self) -> anyhow::Result<()> {
        let mut conn = self.pool.get().await?;

        // ----------------- 加载用户在线状态 -----------------
        let mut cursor = 0u64;
        loop {
            let (next_cursor, keys): (u64, Vec<String>) = cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg("online:user:*")
                .arg("COUNT")
                .arg(100)
                .query_async(&mut conn)
                .await?;

            for key in keys {
                if let Some(key) = key.strip_prefix("online:user:") {
                    let parts: Vec<&str> = key.split(':').collect();
                    if parts.len() >= 4 {
                        let user_id = parts[2].to_string();
                        let device_type: u8 = parts[3].parse().unwrap_or(0);
                        // self.get_online_shard(&user_id).insert(user_id.clone(), DeviceType::from_str_name("WEB".as_str()));
                    }
                }
            }

            if next_cursor == 0 {
                break;
            }
            cursor = next_cursor;
        }

        // ----------------- 加载群组信息 -----------------
        let mut cursor = 0u64;
        loop {
            let (next_cursor, keys): (u64, Vec<String>) = cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg("group:info:*")
                .arg("COUNT")
                .arg(100)
                .query_async(&mut conn)
                .await?;

            for key in keys {
                let json: Option<String> = conn.get(&key).await?;
                if let Some(json) = json {
                    let info: GroupInfo = serde_json::from_str(&json)?;
                    self.local_group_manager.init_group(info);
                }
            }

            if next_cursor == 0 {
                break;
            }
            cursor = next_cursor;
        }

        // ----------------- 加载群组成员和成员元信息 -----------------
        let mut cursor = 0u64;
        loop {
            let (next_cursor, keys): (u64, Vec<String>) = cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg("group:member:*")
                .arg("COUNT")
                .arg(100)
                .query_async(&mut conn)
                .await?;

            for key in keys {
                if let Some(group_id) = key.strip_prefix("group:member:") {
                    let members: Vec<String> = conn.smembers(&key).await.unwrap_or_default();

                    // 获取成员元信息哈希表
                    let meta_key = format!("group:meta:{}", group_id);
                    let metas: HashMap<String, String> =
                        conn.hgetall(&meta_key).await.unwrap_or_default();

                    for uid in members {
                        if let Some(meta_json) = metas.get(&uid) {
                            if let Ok(meta) = serde_json::from_str::<GroupMemberMeta>(meta_json) {
                                // 使用完整信息添加成员到本地缓存
                                self.local_group_manager.add_user(
                                    group_id,
                                    &uid,
                                    Some(meta.mute),
                                    meta.alias.as_deref().unwrap_or(""),
                                    &meta.role,
                                );
                            } else {
                                // fallback: 没有 meta 结构，使用默认 role/alias/mute
                                self.local_group_manager.add_user(
                                    group_id,
                                    &uid,
                                    None,
                                    "",
                                    &GroupRole::Member,
                                );
                            }
                        } else {
                            // fallback: meta 不存在
                            self.local_group_manager.add_user(
                                group_id,
                                &uid,
                                None,
                                "",
                                &GroupRole::Member,
                            );
                        }
                    }
                }
            }

            if next_cursor == 0 {
                break;
            }
            cursor = next_cursor;
        }

        // ----------------- 加载好友信息 -----------------
        cursor = 0u64;
        loop {
            let (next_cursor, keys): (u64, Vec<String>) = cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg("friend:user:*")
                .arg("COUNT")
                .arg(100)
                .query_async(&mut conn)
                .await?;

            for key in keys {
                if let Some(suffix) = key.strip_prefix("friend:user:") {
                    let parts: Vec<&str> = suffix.split(':').collect();
                    if parts.len() == 2 {
                        let agent_id = parts[0];
                        let user_id = parts[1];
                        let full_key = format!("{}:{}", agent_id, user_id);

                        let friends: Vec<String> = conn.smembers(&key).await.unwrap_or_default();
                        let map = DashMap::new();
                        for friend_id in friends {
                            map.insert(friend_id, ());
                        }
                        self.friend_map.insert(full_key, map);
                    }
                }
            }

            if next_cursor == 0 {
                break;
            }
        }

        println!("[UserManager] ✅ 本地缓存初始化完成（在线状态 + 群组信息 + 成员）");
        Ok(())
    }

    pub async fn start_stream_event_consumer(&self) -> anyhow::Result<()> {
        Ok(())
    }
   
    pub async fn clean(&self) -> anyhow::Result<()> {
        Ok(())
    }
    /// 获取用户分片
    pub fn get_online_shard(&self, user_id: &UserId) -> &DashMap<UserId, DashMap<DeviceType, ()>> {
        let hash = fxhash::hash32(user_id.as_bytes());
        &self.local_online_shards[(hash as usize) % SHARD_COUNT]
    }

    pub fn init(&self, instance: UserManager) {
        INSTANCE
            .set(Arc::new(instance))
            .expect("INSTANCE already initialized");
    }

    /// 获取全局实例（未初始化会 panic）
    pub fn get() -> Arc<Self> {
        INSTANCE
            .get()
            .expect("UserManager is not initialized")
            .clone()
    }
}

static INSTANCE: OnceCell<Arc<UserManager>> = OnceCell::new();