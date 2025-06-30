use crate::biz_service::agent_service::AgentService;
use crate::biz_service::friend_service::UserFriendService;
use crate::biz_service::kafka_service::{ByteMessageType, KafkaService};
use crate::entitys::client_entity::ClientInfo;
use crate::manager::common::UserId;
use crate::manager::user_manager_core::{UserManager, UserManagerOpt, USER_ONLINE_TTL_SECS};
use crate::protocol::auth::DeviceType;
use crate::protocol::friend::{EventStatus, FriendEventMessage, FriendEventType, FriendSourceType};
use actix_web::cookie::time::macros::time;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use common::config::AppConfig;
use common::errors::AppError;
use common::repository_util::Repository;
use common::util::common_utils::build_uuid;
use common::util::date_util::now;
use common::ClientTokenDto;
use dashmap::DashMap;
use deadpool_redis::redis::AsyncCommands;
use mongodb::bson::doc;
use tokio::try_join;

#[async_trait]
impl UserManagerOpt for UserManager {
    async fn online(&self, agent_id: &str, user_id: &UserId, device_type: DeviceType) -> anyhow::Result<()> {
        if self.use_local_cache {
            self.get_online_shard(&user_id).insert(user_id.to_string(), device_type);
        }
        let mut conn = self.pool.get().await?;
        let i = device_type as u8;
        let redis_online_key = format!("online:user:agent:{}:{}:{}", agent_id, user_id, i);
        let _: () = conn.set_ex(redis_online_key, i.to_string(), USER_ONLINE_TTL_SECS).await?;
        
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

    async fn add_friend(&self, agent_id: &str, user_id: &UserId, friend_id: &UserId, nickname: &Option<String>, source_type: &FriendSourceType, remark: &Option<String>) -> Result<()> {
        // ---------- 1. 验证用户和好友存在 ----------
        let user_manager = UserManager::get();
        let (client_opt, friend_opt) = try_join!(user_manager.get_user_info(agent_id, user_id), user_manager.get_user_info(agent_id, friend_id))?;

        let client_info = client_opt.ok_or_else(|| anyhow!("用户 {} 不存在", user_id))?;
        let friend_info = friend_opt.ok_or_else(|| anyhow!("好友 {} 不存在", friend_id))?;

        // ---------- 2. 准备昵称 ----------
        let nickname_to_friend = nickname.as_ref().cloned().unwrap_or_else(|| friend_info.alias.clone());
        let nickname_to_user = client_info.alias.clone();

        // ---------- 3. Redis 写入 ----------
        let redis_key = |uid: &UserId| format!("friend:user:{}:{}", agent_id, uid);
        let mut conn = self.pool.get().await?;
        let _: () = conn.sadd(redis_key(user_id), friend_id).await?;
        let _: () = conn.sadd(redis_key(friend_id), user_id).await?;

        // ---------- 4. 本地缓存写入（可选） ----------
        if self.use_local_cache {
            let cache_key = |uid: &UserId| format!("{}:{}", agent_id, uid);
            let insert_cache = |key: String, id: &UserId| {
                self.friend_map.entry(key).or_insert_with(DashMap::new).insert(id.clone(), ());
            };
            insert_cache(cache_key(user_id), friend_id);
            insert_cache(cache_key(friend_id), user_id);
        }

        // ---------- 5. 数据库持久化 ----------
        let friend_service = UserFriendService::get();
         friend_service.add_friend(agent_id, user_id, friend_id, &Some(nickname_to_friend), source_type, remark).await?;
         friend_service.add_friend(agent_id, friend_id, user_id, &Some(nickname_to_user), source_type, remark).await?;

        // ---------- 6. Kafka 消息通知 ----------
        let kafka_service = KafkaService::get();
        let app_config = AppConfig::get();
        let topic = &app_config.kafka.topic_single;
        let time = now();

        // 构造好友事件
        let make_event = |from_uid: &str, to_uid: &str| FriendEventMessage {
            event_id: build_uuid(),
            from_uid: from_uid.to_string(),
            to_uid: to_uid.to_string(),
            event_type: FriendEventType::FriendAddForce as i32,
            message: String::new(),
            status: EventStatus::Done as i32,
            source_type: FriendSourceType::FriendSourceSystem as i32,
            created_at: time,
            updated_at: time,
        };

        // 同步通知双方
        for (form_uid, to_uid) in [(&user_id, &friend_id), (&friend_id, &user_id)] {
            let event = make_event(form_uid, to_uid);
            let node_index=0 as u8;
            if let Err(e) = kafka_service.send_proto(&ByteMessageType::FriendType, &node_index, &event, &event.event_id, topic).await {
                log::warn!("Kafka 消息发送失败 [{}]: {:?}", event.event_id, e);
            }
        }
        Ok(())
    }

    async fn remove_friend(&self, agent_id: &str, user_id: &UserId, friend_id: &UserId) -> Result<()> {
        // ---------- 1. 删除 Redis 中的双向关系 ----------
        let redis_key = |uid: &UserId| format!("friend:user:{}:{}", agent_id, uid);
        let mut conn = self.pool.get().await?;

        let _: () = conn.srem(redis_key(user_id), friend_id).await?;
        let _: () = conn.srem(redis_key(friend_id), user_id).await?;

        // ---------- 2. 删除本地缓存关系 ----------
        if self.use_local_cache {
            let cache_key = |uid: &UserId| format!("{}:{}", agent_id, uid);

            if let Some(map1) = self.friend_map.get(&cache_key(user_id)) {
                map1.remove(friend_id);
            }
            if let Some(map2) = self.friend_map.get(&cache_key(friend_id)) {
                map2.remove(user_id);
            }
        }

        // ---------- 3. 删除数据库中的双向记录 ----------
        let friend_service = UserFriendService::get();
        friend_service.remove_friend(agent_id, user_id, friend_id).await?;
        friend_service.remove_friend(agent_id, friend_id, user_id).await?;

        // ---------- 4. 发送 Kafka 通知（可选） ----------
        let kafka_service = KafkaService::get();
        let app_config = AppConfig::get();
        let topic = &app_config.kafka.topic_single;
        let time = now();

        let make_event = |from_uid: &UserId, to_uid: &UserId| FriendEventMessage {
            event_id: build_uuid(), // 为删除事件生成唯一 ID
            from_uid: from_uid.to_string(),
            to_uid: to_uid.to_string(),
            event_type: FriendEventType::FriendRemove as i32,
            source_type: FriendSourceType::FriendSourceSystem as i32,
            message: String::new(),
            status: EventStatus::Done as i32,
            created_at: time,
            updated_at: time,
        };

        for (from, to) in [(user_id, friend_id), (friend_id, user_id)] {
            let event = make_event(from, to);
            let node_index=0 as u8;
            if let Err(e) = kafka_service.send_proto(&ByteMessageType::FriendType, &node_index, &event, &event.event_id, topic).await {
                log::warn!("Kafka 消息发送失败 [{}]: {:?}", event.event_id, e);
            }
        }

        Ok(())
    }

    async fn is_friend(&self, agent_id: &str, uid: &UserId, friend_id: &UserId) -> Result<bool> {
        // 1. 本地缓存
        if self.use_local_cache {
            let key = format!("{}:{}", agent_id, uid);
            if let Some(map) = self.friend_map.get(&key) {
                if map.contains_key(friend_id) {
                    return Ok(true);
                }
            }
        }

        // 2. Redis 查询
        let redis_key = format!("friend:user:{}:{}", agent_id, uid);
        let mut conn = self.pool.get().await?;
        let exists: bool = conn.sismember(&redis_key, friend_id).await.context("Redis SISMEMBER 查询失败")?;

        if exists {
            return Ok(true);
        }

        // 3. MongoDB 兜底
        let filter = doc! {
            "agent_id": agent_id,
            "uid": uid.to_string(),
            "friend_id": friend_id.to_string(),
            "friend_status": 1 // 限定必须是 Accepted 状态
        };
        let friend_service = UserFriendService::get();
        let exists_in_db = friend_service.dao.find_one(filter).await.map(|opt| opt.is_some()).unwrap_or(false);
        Ok(exists_in_db)
    }

    async fn get_friends(&self, agent_id: &str, user_id: &UserId) -> Result<Vec<UserId>> {
        let cache_key = format!("{}:{}", agent_id, user_id);

        // 1. 本地缓存优先
        if self.use_local_cache {
            if let Some(map) = self.friend_map.get(&cache_key) {
                let friends: Vec<UserId> = map.iter().map(|kv| kv.key().clone()).collect();
                return Ok(friends);
            }
        }

        // 2. Redis 查询
        let redis_key = format!("friend:user:{}:{}", agent_id, user_id);
        let mut conn = self.pool.get().await?;
        let redis_friends: Vec<String> = conn.smembers(&redis_key).await.context("Redis SMEMBERS 获取好友列表失败")?;

        // 如果 Redis 命中，直接返回
        if !redis_friends.is_empty() {
            return Ok(redis_friends);
        }

        let friend_service = UserFriendService::get();
        // 3. MongoDB 兜底查询
        let filter = doc! {
            "agent_id": agent_id,
            "uid": user_id.to_string(),
            "friend_status": 1, // 只取已接受的关系
        };
        let mongo_friends = friend_service.dao.query(filter).await.unwrap_or_default().into_iter().map(|f| f.friend_id).collect::<Vec<UserId>>();

        // 同步回 Redis 和本地缓存（可选）
        if !mongo_friends.is_empty() {
            let _: () = conn.sadd(&redis_key, &mongo_friends).await.unwrap_or_default();

            if self.use_local_cache {
                let map = self.friend_map.entry(cache_key.clone()).or_insert_with(DashMap::new);
                for fid in &mongo_friends {
                    map.insert(fid.clone(), ());
                }
            }
        }

        Ok(mongo_friends)
    }

    async fn friend_block(&self, agent_id: &str, user_id: &UserId, friend_id: &UserId) -> Result<()> {
        let friend_service = UserFriendService::get();

        // ---------- 1. 校验是否有好友关系 ----------
        let mut is_friend = false;

        // 本地缓存
        if self.use_local_cache {
            let key = format!("{}:{}", agent_id, user_id);
            if let Some(map) = self.friend_map.get(&key) {
                if map.contains_key(friend_id) {
                    is_friend = true;
                }
            }
        }

        let mut conn = self.pool.get().await?;

        // Redis 判断
        if !is_friend {
            let redis_key = format!("friend:user:{}:{}", agent_id, user_id);
            let exists: bool = conn.sismember(&redis_key, friend_id).await.unwrap_or(false);
            if exists {
                is_friend = true;
            }
        }

        // Mongo 判断
        let mongo_friend = if !is_friend {
            // 此处也顺便获取记录用于更新
            friend_service.get_friend_detail(agent_id, user_id, friend_id).await?
        } else {
            None
        };

        // 三层都找不到关系才报错
        if !is_friend && mongo_friend.is_none() {
            return Err(anyhow!("用户 {} 与 {} 非好友关系，无法拉黑", user_id, friend_id));
        }
        let friend_info = mongo_friend.unwrap();
        // ---------- 2. 更新数据库标记 is_blocked ----------
        if friend_info.is_blocked {
            // ---------- 3. Redis 添加黑名单集合 ----------
            let redis_block_key = format!("block:user:{}:{}", agent_id, user_id);
            let _: () = conn.sadd(&redis_block_key, friend_id).await?;
            // 已拉黑，无需重复操作
            return Ok(());
        }
        // 提交更新
        friend_service.dao.up_property(&friend_info.id, "is_blocked", true).await?;
        // ---------- 3. Redis 添加黑名单集合 ----------
        let redis_block_key = format!("block:user:{}:{}", agent_id, user_id);
        let _: () = conn.sadd(&redis_block_key, friend_id).await?;
        // ---------- 4. 可选：发送 Kafka 拉黑事件 ----------
        // ---------- 6. Kafka 消息通知 ----------
        let kafka_service = KafkaService::get();
        let app_config = AppConfig::get();
        let topic = &app_config.kafka.topic_single;
        let time = now();

        // 构造好友事件
        let make_event = |from_uid: &str, to_uid: &str| FriendEventMessage {
            event_id: build_uuid(),
            from_uid: from_uid.to_string(),
            to_uid: to_uid.to_string(),
            event_type: FriendEventType::FriendBlock as i32,
            message: String::new(),
            status: EventStatus::Done as i32,
            source_type: FriendSourceType::FriendSourceSystem as i32,
            created_at: time,
            updated_at: time,
        };

        // 同步通知
        for (form_uid, to_uid) in [(user_id, &friend_id)] {
            let event = make_event(form_uid, to_uid);
            let node_index=0 as u8;
            if let Err(e) = kafka_service.send_proto(&ByteMessageType::FriendType, &node_index, &event, &event.event_id, topic).await {
                log::warn!("Kafka 消息发送失败 [{}]: {:?}", event.event_id, e);
            }
        }
        Ok(())
    }

    async fn friend_unblock(&self, agent_id: &str, user_id: &UserId, friend_id: &UserId) -> Result<()> {
        let friend_service = UserFriendService::get();

        // ---------- 1. 校验是否有好友关系 ----------
        let mut is_friend = false;

        // 本地缓存
        if self.use_local_cache {
            let key = format!("{}:{}", agent_id, user_id);
            if let Some(map) = self.friend_map.get(&key) {
                if map.contains_key(friend_id) {
                    is_friend = true;
                }
            }
        }

        let mut conn = self.pool.get().await?;

        // Redis 判断
        if !is_friend {
            let redis_key = format!("friend:user:{}:{}", agent_id, user_id);
            let exists: bool = conn.sismember(&redis_key, friend_id).await.unwrap_or(false);
            if exists {
                is_friend = true;
            }
        }

        // Mongo 判断
        let mongo_friend = if !is_friend {
            // 此处也顺便获取记录用于更新
            friend_service.get_friend_detail(agent_id, user_id, friend_id).await?
        } else {
            None
        };

        // 三层都找不到关系才报错
        if !is_friend && mongo_friend.is_none() {
            return Err(anyhow!("用户 {} 与 {} 非好友关系，无法拉黑", user_id, friend_id));
        }
        let friend_info = mongo_friend.unwrap();
        // ---------- 2. 更新数据库标记 is_blocked ----------
        if !friend_info.is_blocked {
            // ---------- 3. Redis 删除黑名单集合 ----------
            let redis_block_key = format!("block:user:{}:{}", agent_id, user_id);
            let _: () = conn.srem(&redis_block_key, friend_id).await?;
            // 已拉黑，无需重复操作
            return Ok(());
        }
        // 提交更新
        friend_service.dao.up_property(&friend_info.id, "is_blocked", false).await?;
        // ---------- 3. Redis 删除黑名单集合 ----------
        let redis_block_key = format!("block:user:{}:{}", agent_id, user_id);
        let _: () = conn.srem(&redis_block_key, friend_id).await?;
        // ---------- 4. 可选：发送 Kafka 取消拉黑事件 ----------
        let time = now();
        // 构造好友事件
        let make_event = |from_uid: &str, to_uid: &str| FriendEventMessage {
            event_id: build_uuid(),
            from_uid: from_uid.to_string(),
            to_uid: to_uid.to_string(),
            event_type: FriendEventType::FriendUnblock as i32,
            message: String::new(),
            status: EventStatus::Done as i32,
            source_type: FriendSourceType::FriendSourceSystem as i32,
            created_at: time,
            updated_at: time,
        };
        // ---------- 6. Kafka 消息通知 ----------
        let kafka_service = KafkaService::get();
        let app_config = AppConfig::get();
        let topic = &app_config.kafka.topic_single;
        // 同步通知双方
        for (form_uid, to_uid) in [(user_id, &friend_id)] {
            let event = make_event(form_uid, to_uid);
            let node_index=0 as u8;
            if let Err(e) = kafka_service.send_proto(&ByteMessageType::FriendType, &node_index, &event, &event.event_id, topic).await {
                log::warn!("Kafka 消息发送失败 [{}]: {:?}", event.event_id, e);
            }
        }
        Ok(())
    }
}
