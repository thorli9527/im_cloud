use crate::entitys::client_entity::ClientInfo;
use crate::entitys::group_entity::GroupInfo;
use crate::entitys::group_member::{GroupMemberMeta, GroupRole};
use crate::manager::common::{DeviceType, SHARD_COUNT, UserId};
use crate::manager::local_group_manager::{LocalGroupManager, LocalGroupManagerOpt};
use anyhow::{Context, Result};
use common::ClientTokenDto;
use common::errors::AppError;
use common::util::common_utils::build_uuid;
use dashmap::DashMap;
use deadpool_redis::Pool as RedisPool;
use deadpool_redis::redis::{AsyncCommands, cmd};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::sync::Notify;
use tokio::time::sleep;
use crate::biz_service::user_friend_service::UserFriendService;
use crate::entitys::user_friend::FriendSourceType;

const MAX_CLEAN_COUNT: usize = 100;
const USER_ONLINE_TTL_SECS: u64 = 30;
const STREAM_KEY: &str = "user:events";
const CONSUMER_GROUP: &str = "user_events_group";
const CONSUMER_NAME: &str = "user_manager";

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum UserEvent {
    GroupLeave {
        group_id: String,
        user_id: UserId,
    },
    GroupJoin {
        group_id: String,
        user_id: UserId,
        mute: Option<bool>,
        alias: String,
        role: GroupRole,
    },
    Online {
        user_id: UserId,
        device: DeviceType,
    },
    Offline {
        user_id: UserId,
        device: DeviceType,
    },
}

/// 全局用户管理器
#[derive(Debug, Clone)]
/// `UserManager` 管理用户在线状态、群组缓存、初始化状态及 Redis 通信。
pub struct UserManager {
    /// Redis 连接池，用于访问用户状态、群组数据、事件队列等。
    pool: RedisPool,

    /// 本地用户在线状态缓存，按分片存储，使用 DashMap 支持多线程并发访问。
    /// 每个分片是一个 DashMap，key 为 user_id，value 为占位单元类型 `()`
    /// 用于快速判断用户是否在线，减少 Redis 访问。
    local_online_shards: Arc<Vec<DashMap<UserId, DeviceType>>>,

    /// 本地群组缓存，用于存储每个群组的用户列表等信息，支持分片访问。
    /// 提高群组相关操作性能，降低 Redis 压力。
    local_group_manager: Arc<LocalGroupManager>,

    /// 标记是否已初始化，避免重复初始化。
    /// 使用 Arc + AtomicBool 保证跨线程安全修改。
    is_initialized: Arc<AtomicBool>,

    /// 初始化通知器，未完成初始化时异步任务可以 await 等待。
    /// 配合 `is_initialized` 实现任务级初始化阻塞。
    init_notify: Arc<Notify>,
    /// 全局用户好友关系映射，使用 DashMap 支持多线程并发访问。
    friend_map: Arc<DashMap<String, DashMap<UserId, ()>>>,
    /// 是否启用本地缓存。
    /// 如果为 false，将直接查询 Redis，适用于测试或轻量部署模式。
    use_local_cache: bool,
}

/// 用户管理核心行为抽象接口
pub trait UserManagerOpt: Send + Sync {
    /// 将用户标记为在线，并进行必要的缓存更新和事件通知
    async fn online(&self, agent_id: &str, user_id: &UserId, device_type: DeviceType) -> Result<()>;
    /// 检查用户是否在线，返回 true 或 false
    async fn is_online(&self, agent_id: &str, user_id: &UserId) -> Result<bool>;
    /// 将用户标记为离线，更新缓存并通知其他服务
    async fn offline(&self, agent_id: &str, user_id: &UserId, device_type: DeviceType) -> Result<()>;
    /// 同步指定用户的信息（例如从数据库或Redis加载最新数据到本地缓存）
    async fn sync_user(&self, user: ClientInfo) -> Result<()>;
    /// 移除指定用户缓存
    async fn remove_user(&self, agent_id: &str, user_id: &UserId) -> Result<()>;
    /// 获取用户的在线状态
    async fn get_user_info(&self, agent_id: &str, user_id: &UserId) -> Result<Option<ClientInfo>>;
    /// 构建用户的访问令牌（例如JWT或其他形式的认证令牌）
    async fn build_token(&self, agent_id: &str, user_id: &UserId, device_type: DeviceType) -> Result<String>;
    /// 删除用户的访问令牌
    async fn delete_token(&self, token: &str) -> Result<()>;
    /// 验证用户的访问令牌，返回用户ID或错误
    async fn verify_token(&self, token: &str) -> Result<bool>;
    /// 获取用户的访问令牌信息
    async fn get_client_token(&self, token: &str) -> Result<ClientTokenDto>;
    /// 根据令牌查找用户信息
    async fn find_user_by_token(&self, token: &str) -> Result<Option<ClientInfo>>;
    /// 添加好友关系
    async fn add_friend(&self, agent_id: &str, user_id: &UserId, friend_id: &UserId,nickname: &Option<String>, source_type: &FriendSourceType,remark:&Option<String>) -> Result<()>;
    /// 移除好友关系
    async fn remove_friend(&self, agent_id: &str, user_id: &UserId, friend_id: &UserId) -> Result<()>;
    /// 检查用户是否是好友关系
    async fn is_friend(&self, agent_id: &str, user_id: &UserId, friend_id: &UserId) -> Result<bool>;
    /// 获取用户的好友列表
    async fn get_friends(&self, agent_id: &str, user_id: &UserId) -> Result<Vec<UserId>>;
}

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
                let exists: Vec<String> = cmd("KEYS").arg(&pattern).query_async(&mut conn).await.unwrap_or_default();

                if exists.is_empty() {
                    shard.remove(&user_id);
                    removed_count += 1;
                    println!("[UserManager] 🧹 清理离线用户缓存: {}", user_id);
                }
            }
        }

        println!("[UserManager] ✅ 在线缓存清理完成，总计 {} 个用户", removed_count);
        Ok(removed_count)
    }
    pub async fn initialize_from_redis(&self) -> Result<()> {
        let mut conn = self.pool.get().await?;

        // ----------------- 加载用户在线状态 -----------------
        let mut cursor = 0u64;
        loop {
            let (next_cursor, keys): (u64, Vec<String>) = cmd("SCAN").arg(cursor).arg("MATCH").arg("online:user:*").arg("COUNT").arg(100).query_async(&mut conn).await?;

            for key in keys {
                if let Some(key) = key.strip_prefix("online:user:") {
                    let parts: Vec<&str> = key.split(':').collect();
                    if parts.len() >= 4 {
                        let user_id = parts[2].to_string();
                        let device_type: u8 = parts[3].parse().unwrap_or(0);
                        self.get_online_shard(&user_id).insert(user_id.clone(), DeviceType::from(device_type));
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
            let (next_cursor, keys): (u64, Vec<String>) = cmd("SCAN").arg(cursor).arg("MATCH").arg("group:info:*").arg("COUNT").arg(100).query_async(&mut conn).await?;

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
            let (next_cursor, keys): (u64, Vec<String>) = cmd("SCAN").arg(cursor).arg("MATCH").arg("group:member:*").arg("COUNT").arg(100).query_async(&mut conn).await?;

            for key in keys {
                if let Some(group_id) = key.strip_prefix("group:member:") {
                    let members: Vec<String> = conn.smembers(&key).await.unwrap_or_default();

                    // 获取成员元信息哈希表
                    let meta_key = format!("group:meta:{}", group_id);
                    let metas: HashMap<String, String> = conn.hgetall(&meta_key).await.unwrap_or_default();

                    for uid in members {
                        if let Some(meta_json) = metas.get(&uid) {
                            if let Ok(meta) = serde_json::from_str::<GroupMemberMeta>(meta_json) {
                                // 使用完整信息添加成员到本地缓存
                                self.local_group_manager.add_user(group_id, &uid, Some(meta.mute), meta.alias.as_deref().unwrap_or(""), &meta.role);
                            } else {
                                // fallback: 没有 meta 结构，使用默认 role/alias/mute
                                self.local_group_manager.add_user(group_id, &uid, None, "", &GroupRole::Member);
                            }
                        } else {
                            // fallback: meta 不存在
                            self.local_group_manager.add_user(group_id, &uid, None, "", &GroupRole::Member);
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
        // ----------------- 加载好友信息 -----------------
        let mut cursor = 0u64;
        loop {
            let (next_cursor, keys): (u64, Vec<String>) = cmd("SCAN").arg(cursor).arg("MATCH").arg("friend:user:*").arg("COUNT").arg(100).query_async(&mut conn).await?;

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

    pub async fn start_stream_event_consumer(&self) -> Result<()> {
        // let pool = self.pool.clone();
        // let shards = self.local_online_shards.clone();
        // let groups = self.local_group_manager.clone();
        // // let node_id = self.node_id;
        // // let node_total = self.node_total;
        //
        // tokio::spawn(async move {
        //     // 初始化消费者组（幂等）
        //     if let Ok(mut conn) = pool.get().await {
        //         let _ = cmd("XGROUP")
        //             .arg("CREATE")
        //             .arg(STREAM_KEY)
        //             .arg(CONSUMER_GROUP)
        //             .arg("0")
        //             .arg("MKSTREAM")
        //             .query_async::<()>(&mut conn)
        //             .await
        //             .or_else(|e| if e.to_string().contains("BUSYGROUP") { Ok(()) } else { Err(e) });
        //     }
        //
        //     loop {
        //         if let Ok(mut conn) = pool.get().await {
        //             let opts = StreamReadOptions::default().group(CONSUMER_GROUP, CONSUMER_NAME).count(10).block(5000);
        //             let result = conn.set_options::<_, _, StreamReadReply>(&[STREAM_KEY], &[">"], &opts).await;
        //
        //             if let Ok(reply) = result {
        //                 for stream in reply.keys {
        //                     for entry in stream.ids {
        //                         if let Some(payload_value) = entry.map.get("payload") {
        //                             // 解析 Redis value 为 String
        //                             let payload_str: String = match from_redis_value(payload_value) {
        //                                 Ok(val) => val,
        //                                 Err(e) => {
        //                                     eprintln!("[RedisUserManager] ❌ payload 类型错误: {:?}", e);
        //                                     continue;
        //                                 }
        //                             };
        //
        //                             // 解析 JSON -> UserEvent
        //                             match serde_json::from_str::<UserEvent>(&payload_str) {
        //                                 Ok(event) => {
        //                                     handle_user_event(&event, &shards, &groups).await;
        //
        //                                     // ACK 消息
        //                                     let _: RedisResult<()> = conn.xack(STREAM_KEY, CONSUMER_GROUP, &[&entry.id]).await;
        //                                 }
        //                                 Err(e) => {
        //                                     eprintln!("[RedisUserManager] ❗️事件反序列化失败: {:?}, 内容: {}", e, payload_str);
        //                                 }
        //                             }
        //                         }
        //                     }
        //                 }
        //             }
        //         }
        //
        //         // 防止空转 CPU 爆炸
        //         sleep(Duration::from_millis(200)).await;
        //     }
        // });

        Ok(())
    }

    pub async fn clean(&self) -> Result<()> {
        Ok(())
    }
    /// 获取用户分片
    fn get_online_shard(&self, user_id: &UserId) -> &DashMap<UserId, DeviceType> {
        let hash = fxhash::hash32(user_id.as_bytes());
        &self.local_online_shards[(hash as usize) % SHARD_COUNT]
    }

    pub fn init(&self, instance: UserManager) {
        INSTANCE.set(Arc::new(instance)).expect("INSTANCE already initialized");
    }

    /// 获取全局实例（未初始化会 panic）
    pub fn get() -> Arc<Self> {
        INSTANCE.get().expect("UserManager is not initialized").clone()
    }
}

impl UserManagerOpt for UserManager {
    async fn online(&self, agent_id: &str, user_id: &UserId, device_type: DeviceType) -> anyhow::Result<()> {
        if self.use_local_cache {
            self.get_online_shard(&user_id).insert(user_id.to_string(), device_type);
        }
        let mut conn = self.pool.get().await?;
        let i = device_type as u8;
        let redis_online_key = format!("online:user:agent:{}:{}:{}", agent_id, user_id, i);
        let _: () = conn.set_ex(redis_online_key, i.to_string(), USER_ONLINE_TTL_SECS).await?;
        //let payload = serde_json::to_string(&UserEvent::Online { user_id: user_id.to_string(), device })?;
        //发消息 上线
        Ok(())
    }

    async fn offline(&self, agent_id: &str, user_id: &UserId, device_type: DeviceType) -> anyhow::Result<()> {
        if self.use_local_cache {
            self.get_online_shard(&user_id).insert(user_id.to_string(), device_type);
        }
        let mut conn = self.pool.get().await?;
        let i = device_type as u8;
        let redis_online_key = format!("online:user:agent:{}:{}:{}", agent_id, user_id, i);
        let _: () = conn.del(redis_online_key).await?;
        //let payload = serde_json::to_string(&UserEvent::Online { user_id: user_id.to_string(), device })?;
        //发消息 下线
        Ok(())
    }

    async fn is_online(&self, agent_id: &str, user_id: &UserId) -> anyhow::Result<bool> {
        if self.use_local_cache {
            let shard = self.get_online_shard(&user_id);
            return Ok(shard.contains_key(user_id));
        }
        let mut conn = self.pool.get().await?;
        let redis_online_key = format!("online:user:agent:{}:{}", agent_id, user_id);
        let exists: bool = conn.exists(redis_online_key).await?;
        Ok(exists)
    }

    async fn sync_user(&self, _user: ClientInfo) -> anyhow::Result<()> {
        let user_id = &_user.uid;
        let mut conn = self.pool.get().await?;
        let user_info_json = serde_json::to_string(&_user)?;
        let string = format!("agent:{}:client:{}", &_user.agent_id, user_id);
        let _: () = conn.set(string, user_info_json).await?;
        Ok(())
    }

    async fn remove_user(&self, agent_id: &str, user_id: &UserId) -> Result<()> {
        if self.use_local_cache {
            self.get_online_shard(user_id).remove(user_id.as_str());
        }
        let mut conn = self.pool.get().await?;
        let redis_online_key = format!("online:user:agent:{}:{}", agent_id, user_id);
        let _: () = conn.del(redis_online_key).await.context("删除在线状态失败")?;
        let key = format!("agent:{}:client:{}", agent_id, user_id);
        let _: () = conn.del(&key).await.context("删除用户缓存失败")?;
        // 发消息 册除用户
        Ok(())
    }

    async fn get_user_info(&self, agent_id: &str, user_id: &UserId) -> Result<Option<ClientInfo>> {
        let mut conn = self.pool.get().await?;
        let key = format!("agent:{}:client:{}", agent_id, user_id);
        let json: Option<String> = conn.get(&key).await?;

        let result = match json {
            Some(data) => {
                let client: ClientInfo = serde_json::from_str(&data)?;
                Some(client)
            }
            None => None,
        };

        Ok(result)
    }

    async fn build_token(&self, agent_id: &str, user_id: &UserId, device_type: DeviceType) -> Result<String> {
        let token_key = build_uuid();
        let key = format!("token:{}", token_key);
        let mut conn = self.pool.get().await?;
        let dto = ClientTokenDto { agent_id: agent_id.to_string(), user_id: user_id.clone(), device_type: device_type as u8 };
        let token_str = serde_json::to_string(&dto).context("序列化 TokenDto 失败")?;
        let _: () = conn.set_ex(key, token_str, 3600).await?;
        Ok(token_key)
    }

    async fn delete_token(&self, token: &str) -> Result<()> {
        let key = format!("token:{}", token);
        let mut conn = self.pool.get().await?;
        let _: () = conn.del(&key).await.context("删除 token 失败")?;
        Ok(())
    }

    async fn verify_token(&self, token: &str) -> Result<bool> {
        let mut conn = self.pool.get().await?;
        let key = format!("token:{}", token);
        let exists: bool = conn.exists(&key).await.context("检查 token 是否存在失败")?;
        Ok(exists)
    }

    async fn get_client_token(&self, token: &str) -> Result<ClientTokenDto> {
        let mut conn = self.pool.get().await?;
        let key = format!("token:{}", token);
        let json: String = conn.get(&key).await.context("获取 token 数据失败")?;
        let dto: ClientTokenDto = serde_json::from_str(&json).context("反序列化 ClientTokenDto 失败")?;
        Ok(dto)
    }

    async fn find_user_by_token(&self, token: &str) -> Result<Option<ClientInfo>> {
        let mut conn = self.pool.get().await?;
        let key = format!("token:{}", token);
        let json: Option<String> = conn.get(&key).await?;
        if let Some(data) = json {
            let dto: ClientTokenDto = serde_json::from_str(&data).context("反序列化 ClientTokenDto 失败")?;
            let result = self
                .get_user_info(&dto.agent_id, &dto.user_id)
                .await
                .context("获取用户信息失败")?
                .ok_or_else(|| AppError::BizError("用户不存在".to_string()))?;
            return Ok(Some(result));
        }
        Ok(None)
    }

    async fn add_friend(&self, agent_id: &str, user_id: &UserId, friend_id: &UserId,nickname: &Option<String>, source_type: &FriendSourceType,remark:&Option<String>) -> Result<()> {
        let key1 = format!("friend:user:{}:{}", agent_id, user_id);
        let key2 = format!("friend:user:{}:{}", agent_id, friend_id);
        let mut conn = self.pool.get().await?;

        // 写入 Redis 双向关系
        let _: () = conn.sadd(&key1, friend_id).await?;
        let _: () = conn.sadd(&key2, user_id).await?;

        if self.use_local_cache {
            let key1 = format!("{}:{}", agent_id, user_id);
            let key2 = format!("{}:{}", agent_id, friend_id);

            let map1 = self.friend_map.entry(key1).or_insert_with(DashMap::new);
            map1.insert(friend_id.clone(), ());

            let map2 = self.friend_map.entry(key2).or_insert_with(DashMap::new);
            map2.insert(user_id.clone(), ());
        }
        let friend_service=UserFriendService::get();
        friend_service.add_friend(agent_id, user_id, friend_id,nickname,source_type,remark).await?;
        Ok(())
    }

    async fn remove_friend(&self, agent_id: &str, user_id: &UserId, friend_id: &UserId) -> Result<()> {
        let key1 = format!("friend:user:{}:{}", agent_id, user_id);
        let key2 = format!("friend:user:{}:{}", agent_id, friend_id);
        let mut conn = self.pool.get().await?;

        // 删除 Redis 中的双向关系
        let _: () = conn.srem(&key1, friend_id).await?;
        let _: () = conn.srem(&key2, user_id).await?;

        if self.use_local_cache {
            let key1 = format!("{}:{}", agent_id, user_id);
            let key2 = format!("{}:{}", agent_id, friend_id);

            if let Some(map1) = self.friend_map.get(&key1) {
                map1.remove(friend_id);
            }
            if let Some(map2) = self.friend_map.get(&key2) {
                map2.remove(user_id);
            }
        }

        Ok(())
    }

    async fn is_friend(&self, agent_id: &str, user_id: &UserId, friend_id: &UserId) -> Result<bool> {
        // 1. 本地缓存查询
        if self.use_local_cache {
            let key = format!("{}:{}", agent_id, user_id);
            if let Some(map) = self.friend_map.get(&key) {
                if map.contains_key(friend_id) {
                    return Ok(true);
                }
            }
        }

        // 2. Redis 查询
        let redis_key = format!("friend:user:{}:{}", agent_id, user_id);
        let mut conn = self.pool.get().await?;
        let exists: bool = conn.sismember(&redis_key, friend_id).await.context("Redis SISMEMBER 查询失败")?;

        Ok(exists)
    }

    async fn get_friends(&self, agent_id: &str, user_id: &UserId) -> Result<Vec<UserId>> {
        let key = format!("{}:{}", agent_id, user_id);

        // 1. 本地缓存
        if self.use_local_cache {
            if let Some(map) = self.friend_map.get(&key) {
                let friends: Vec<UserId> = map.iter().map(|kv| kv.key().clone()).collect();
                return Ok(friends);
            }
        }
        // 2. Redis 获取好友集合
        let redis_key = format!("friend:user:{}:{}", agent_id, user_id);
        let mut conn = self.pool.get().await?;
        let friend_ids: Vec<String> = conn.smembers(&redis_key).await.context("Redis SMEMBERS 获取好友列表失败")?;
        Ok(friend_ids)
    }
}

// 全局单例实例
static INSTANCE: OnceCell<Arc<UserManager>> = OnceCell::new();

async fn handle_user_event(event: &UserEvent, shards: &Vec<DashMap<UserId, DeviceType>>, groups: LocalGroupManager) {
    match event {
        UserEvent::Online { user_id, device } => {
            // let shard = &shards[hash_user(user_id) % SHARD_COUNT];
            // shard.insert(user_id.to_string(), device.clone());
        }
        UserEvent::Offline { user_id, device } => {
            // let shard = &shards[hash_user(user_id) % SHARD_COUNT];
            // shard.remove(user_id);
        }
        UserEvent::GroupJoin { group_id, user_id, mute, alias, role } => {
            let current_mute = mute.unwrap_or(false);
            groups.add_user(&group_id, &user_id, Option::Some(current_mute), &alias, &role);
        }
        UserEvent::GroupLeave { group_id, user_id } => {
            groups.remove_user(&group_id, &user_id);
        }
    }
}
