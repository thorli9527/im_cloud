use deadpool_redis::redis::{cmd, AsyncCommands};
use once_cell::sync::OnceCell;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use twox_hash::XxHash64;
use crate::protocol::common::{GroupEntity, GroupMemberEntity};

pub const MAX_CLEAN_COUNT: usize = 100;
pub const USER_ONLINE_TTL_SECS: u64 = 300;
pub const STREAM_KEY: &str = "user:events";
pub const CONSUMER_GROUP: &str = "user_events_group";
pub const CONSUMER_NAME: &str = "user_manager";
const SHARD_SIZE: usize = 64;
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
    pub fn new(pool: RedisPool) -> Self {
        let manager = Self {
            pool,
            is_initialized: Arc::new(AtomicBool::new(false)),
            init_notify: Arc::new(Notify::new()),
            friend_map: Arc::new(DashMap::<String, DashMap<UserId, ()>>::new()),
        };
        manager.init(manager.clone());
        return manager;
    }
    /// 初始化用户管理器   计算分片索引
 
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
                    let info: GroupEntity = serde_json::from_str(&json)?;
                    // self.local_group_manager.init_group(info);
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
                            if let Ok(meta) = serde_json::from_str::<GroupMemberEntity>(meta_json) {
                                // // 使用完整信息添加成员到本地缓存
                                // self.local_group_manager.add_user(
                                //     group_id,
                                //     &uid,
                                //     Some(meta.mute),
                                //     meta.alias.as_deref().unwrap_or(""),
                                //     &meta.role,
                                // );
                            } else {
                                // fallback: 没有 meta 结构，使用默认 role/alias/mute
                                // self.local_group_manager.add_user(
                                //     group_id,
                                //     &uid,
                                //     None,
                                //     "",
                                //     &GroupRole::Member,
                                // );
                            }
                        } else {
                            // fallback: meta 不存在
                            // self.local_group_manager.add_user(
                            //     group_id,
                            //     &uid,
                            //     None,
                            //     "",
                            //     &GroupRole::Member,
                            // );
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
    // 获取 Redis 分片索引
    pub fn get_redis_shard_index(&self, user_id: &str) -> usize {
        let key = format!("{}",  user_id);
        let mut hasher = XxHash64::with_seed(0);  // 可选固定种子
        key.hash(&mut hasher);
        (hasher.finish() % SHARD_SIZE as u64) as usize
    }
    // 生成在线状态键
    pub fn make_online_key(&self, user_id: &UserId) -> (String, String) {
        let shard = self.get_redis_shard_index( user_id);
        let redis_key = format!("online:user:shard:{}", shard);
        let redis_field = format!("{}",  user_id);
        (redis_key, redis_field)
    }
    pub async fn start_stream_event_consumer(&self) -> anyhow::Result<()> {
        Ok(())
    }

    pub async fn clean(&self) -> anyhow::Result<()> {
        Ok(())
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
