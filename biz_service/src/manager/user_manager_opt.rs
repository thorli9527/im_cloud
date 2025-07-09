use crate::biz_service::agent_service::AgentService;
use crate::biz_service::client_service::ClientService;
use crate::biz_service::friend_service::UserFriendService;
use crate::biz_service::kafka_service::KafkaService;
use crate::entitys::client_entity::ClientInfo;
use crate::manager::common::UserId;
use crate::manager::user_manager_core::{USER_ONLINE_TTL_SECS, UserManager, UserManagerOpt};
use crate::protocol::auth::{DeviceType, LoginRespMsg, LogoutRespMsg, OfflineStatueMsg, OnlineStatusMsg};
use crate::protocol::common::ByteMessageType;
use crate::protocol::friend::{EventStatus, FriendEventMsg, FriendEventType, FriendSourceType};
use actix_web::cookie::time::macros::time;
use actix_web::web::get;
use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use common::ClientTokenDto;
use common::config::AppConfig;
use common::errors::AppError;
use common::repository_util::Repository;
use common::util::common_utils::{build_md5_with_key, build_snow_id, build_uuid};
use common::util::date_util::now;
use dashmap::DashMap;
use deadpool_redis::redis::AsyncCommands;
use futures_util::TryFutureExt;
use mongodb::bson::doc;
use tokio::try_join;
use crate::protocol::status::AckMsg;
pub const TOKEN_EXPIRE_SECS: u64 = 60 * 60 * 24 * 7;
#[async_trait]
impl UserManagerOpt for UserManager {
    async fn login(
        &self,
        message_id: &u64,
        app_key: &str,
        user_name: &str,
        password: &str,
        device_type: &DeviceType,
    ) -> anyhow::Result<String> {
        let agent_service = AgentService::get();

        let agent_info = agent_service.find_by_app_key(app_key).await?;
        let agent_id = agent_info.id.clone();
        let client_info = self.get_user_info_by_name(&agent_id, user_name).await?;

        if client_info.is_none() {
            return Err(anyhow::anyhow!("user.or.password.error"));
        }
        let app_config = AppConfig::get();
        let string = build_md5_with_key(password, &app_config.sys.md5_key);
        let client = client_info.unwrap();
        if &client.password.unwrap() != &string {
            return Err(anyhow::anyhow!("user.or.password.error"));
        }
        let user_id = client.id.clone() as UserId;
        self.online(&agent_id, &user_id, device_type).await?;

        let token = self.build_token(&agent_id, &user_id, device_type).await?;


        let kafka_service = KafkaService::get();
        let node_index: &u8= &0;
        // 发送登录响应消息
        kafka_service.send_proto(&ByteMessageType::LoginRespMsgType, &node_index, &LoginRespMsg {
            message_id: Some(message_id.clone()),
            token: token.clone(),
            expires_at: now() as u64,
        }, &build_uuid(), &AppConfig::get().kafka.topic_group).await?;
        Ok(token)
    }

    async fn logout(
        &self,
        message_id: &u64,
        agent_id: &str,
        user_id: &UserId,
        device_type: &DeviceType,
    ) -> anyhow::Result<()> {
        self.offline(agent_id, user_id, device_type).await;
        let node_index: &u8= &0;
        let kafka_service = KafkaService::get();
        // 发送登出响应消息
        kafka_service.send_proto(&ByteMessageType::LogoutRespMsgType, &node_index, &LogoutRespMsg {
            message_id: Some(message_id.clone()),
        }, &build_uuid(), &AppConfig::get().kafka.topic_group).await?;
        Ok(())
    }
    async fn online(
        &self,
        agent_id: &str,
        user_id: &UserId,
        device_type: &DeviceType,
    ) -> Result<()> {
        let mut is_first_online = false;

        // 本地缓存判断是否首次上线
        if self.use_local_cache {
            let shard = self.get_online_shard(user_id);

            let user_map = shard.entry(user_id.clone()).or_insert_with(DashMap::new);

            //判断是否首次上线
            if user_map.is_empty() {
                is_first_online = true;
            }

            user_map.insert(device_type.clone(), ());
        } else {
            // 如果未启用本地缓存，则通过 Redis 判断是否首次上线
            is_first_online = self.is_all_device_offline(agent_id, user_id).await?;
        }

        // 设置 Redis 在线标记
        let mut conn = self.pool.get().await?;
        let key = format!("online:user:agent:{}:{}:{}", agent_id, user_id, *device_type as u8);
        let _: () = conn.set_ex(&key, "1", USER_ONLINE_TTL_SECS).await?;

        // 处理首次上线逻辑
        if is_first_online {
            let kafka = KafkaService::get(); // 假设你有 KafkaService::send_offline_event
            // 发送 组MQ 在线消息
            let offline_msg=OnlineStatusMsg {
                message_id: Some(build_snow_id()),
                uid: user_id.clone(),
                device_type: device_type.clone() as i32,
                client_id: "".to_string(),
                login_time: now(),
            };
            let node_index: &u8= &0;
            let message_id = &offline_msg.message_id.unwrap().to_string();
            let topic=&AppConfig::get().kafka.topic_group;
            kafka.send_proto(&ByteMessageType::LogoutRespMsgType, node_index, &offline_msg, message_id,&topic).await?;

        }

        Ok(())
    }


    async fn offline(
        &self,
        agent_id: &str,
        user_id: &UserId,
        device_type: &DeviceType,
    ) -> Result<()> {
        if self.use_local_cache {
            if let Some(mut user_map_ref) = self.get_online_shard(user_id).get_mut(user_id) {
                user_map_ref.remove(device_type);
                if user_map_ref.is_empty() {
                    self.get_online_shard(user_id).remove(user_id);
                }
            }
        }

        let mut conn = self.pool.get().await?;
        let key = format!("online:user:agent:{}:{}:{}", agent_id, user_id, *device_type as u8);
        let _: () = conn.del(&key).await?;

        // 👇 判断是否所有设备都离线
        let is_all_offline = self.is_all_device_offline(agent_id, user_id).await?;
        // 👇 所有设备下线后，发送 MQ 消息
        if is_all_offline {
            let kafka = KafkaService::get(); // 假设你有 KafkaService::send_offline_event
            let offline_msg=OfflineStatueMsg {
                message_id: Some(build_snow_id()),
                uid: user_id.clone(),
                device_type: DeviceType::All as i32,
                client_id: "".to_string(),
                logout_time: now(),
                reason: "".to_string(),
            };
            let node_index: &u8= &0;
            let message_id = &offline_msg.message_id.unwrap().to_string();
            let topic=&AppConfig::get().kafka.topic_group;
            kafka.send_proto(&ByteMessageType::LogoutRespMsgType, node_index, &offline_msg, message_id,&topic).await?;
        }
        Ok(())
    }

    async fn is_online(&self, agent_id: &str, user_id: &UserId) -> Result<bool> {
        if self.use_local_cache {
            if let Some(user_map) = self.get_online_shard(user_id).get(user_id) {
                return Ok(!user_map.is_empty());
            } else {
                return Ok(false);
            }
        }

        let mut conn = self.pool.get().await?;
        for i in 0..=5 {
            let key = format!("online:user:agent:{}:{}:{}", agent_id, user_id, i);
            if conn.exists(&key).await? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    async fn get_online_devices(
        &self,
        agent_id: &str,
        user_id: &UserId,
    ) -> Result<Vec<DeviceType>> {
        let mut online_devices = vec![];

        if self.use_local_cache {
            let shard = self.get_online_shard(user_id);
            if let Some(map) = shard.get(user_id) {
                for dt in map.iter() {
                    online_devices.push(dt.key().clone());
                }
                return Ok(online_devices);
            }
        }

        let mut conn = self.pool.get().await?;
        for i in 0..=5 {
            let key = format!("online:user:agent:{}:{}:{}", agent_id, user_id, i);
            if conn.exists(&key).await? {
                if let Some(device) = DeviceType::from_i32(i) {
                    online_devices.push(device);
                }
            }
        }

        Ok(online_devices)
    }

    async fn is_all_device_offline(&self, agent_id: &str, user_id: &UserId) -> Result<bool> {
        // 优先从本地缓存判断（如启用）
        if self.use_local_cache {
            let shard = self.get_online_shard(user_id);
            if let Some(device_map) = shard.get(user_id) {
                if !device_map.is_empty() {
                    return Ok(false);
                }
            }
        }

        // Redis 兜底判断
        let mut conn = self.pool.get().await?;
        for i in 0..=5 {
            let key = format!("online:user:agent:{}:{}:{}", agent_id, user_id, i);
            if conn.exists(&key).await? {
                return Ok(false);
            }
        }

        Ok(true)
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
        let _: () = conn
            .del(redis_online_key)
            .await
            .context("删除在线状态失败")?;
        let key = format!("agent:{}:client:{}", agent_id, user_id);
        let _: () = conn.del(&key).await.context("删除用户缓存失败")?;
        // 发消息 册除用户
        Ok(())
    }

    /// 优先查 Redis，否则从 MongoDB 兜底并写入 Redis
    async fn get_user_info(&self, agent_id: &str, user_id: &UserId) -> Result<Option<ClientInfo>> {
        let mut conn = self.pool.get().await?;
        let key = format!("agent:{}:client:{}", agent_id, user_id);

        // 1. 尝试从 Redis 获取
        if let Ok(Some(cached_json)) = conn.get::<_, Option<String>>(&key).await {
            let parsed = serde_json::from_str(&cached_json)?;
            return Ok(Some(parsed));
        }

        // 2. Redis 未命中，从 MongoDB 查找
        let client_service = ClientService::get();
        let client_opt = client_service
            .dao
            .find_one(doc! { "agentId": agent_id, "userId": user_id })
            .await?;

        // 3. 如果查到，写回 Redis（可设置过期）
        if let Some(ref client) = client_opt {
            let json = serde_json::to_string(client)?;
            let _: () = conn.set_ex(&key, json, 3600).await?;
        }

        Ok(client_opt)
    }

    async fn get_user_info_by_name(
        &self,
        agent_id: &str,
        name: &str,
    ) -> Result<Option<ClientInfo>> {
        let client_service = ClientService::get();
        let client_opt = client_service
            .dao
            .find_one(doc! { "agent_id": agent_id, "username": name })
            .await?;
        return Ok(client_opt);
    }

    /// 构建 Token：写入主数据、索引、集合（反向查找用）
    async fn build_token(
        &self,
        agent_id: &str,
        uid: &UserId,
        device_type: &DeviceType,
    ) -> Result<String> {
        let token_key = build_uuid();
        let token_data_key = format!("token:{}", token_key);
        let token_index_key = format!("token:index:{}:{}:{}", agent_id, uid, *device_type as u8);
        let token_set_key = format!("token:uid:{}:{}", agent_id, uid);

        let dto = ClientTokenDto {
            agent_id: agent_id.to_string(),
            uid: uid.clone(),
            device_type: *device_type as u8,
        };

        let token_str = serde_json::to_string(&dto).context("序列化 ClientTokenDto 失败")?;
        let mut conn = self.pool.get().await?;

        // 主数据 & 单设备索引
        let _: () = conn.set_ex(&token_data_key, token_str, TOKEN_EXPIRE_SECS).await?;
        let _: () = conn.set_ex(&token_index_key, &token_key, TOKEN_EXPIRE_SECS).await?;

        // 添加到 uid 所有 token 集合（便于注销所有 token）
        let _: () = conn.sadd(&token_set_key, &token_key).await?;
        let _: () = conn.expire(&token_set_key, TOKEN_EXPIRE_SECS as i64).await?;

        Ok(token_key)
    }
    /// 获取指定 uid + 设备的 token（单设备支持）
    async fn get_token_by_uid_device(
        &self,
        agent_id: &str,
        uid: &UserId,
        device_type: DeviceType,
    ) -> Result<Option<String>> {
        let mut conn = self.pool.get().await?;
        let index_key = format!("token:index:{}:{}:{}", agent_id, uid, device_type as u8);
        let token: Option<String> = conn.get(&index_key).await.context("获取 token 索引失败")?;
        Ok(token)
    }
    /// 删除指定 token（包括索引 + 主数据 + 集合成员）
    /// 删除指定 token（包括索引 + 主数据 + 集合成员）
    async fn delete_token(&self, token: &str) -> Result<()> {
        let mut conn = self.pool.get().await?;
        let token_key = format!("token:{}", token);

        if let Ok(json) = conn.get::<_, String>(&token_key).await {
            if let Ok(dto) = serde_json::from_str::<ClientTokenDto>(&json) {
                let index_key = format!(
                    "token:index:{}:{}:{}",
                    dto.agent_id, dto.uid, dto.device_type
                );
                let token_set_key = format!("token:uid:{}:{}", dto.agent_id, dto.uid);

                let _: () = conn.del(index_key).await?;
                let _: () = conn.srem(token_set_key, token).await?;
            }
        }

        let _: () = conn
            .del(&token_key)
            .await
            .context("删除 token 主数据失败")?;
        Ok(())
    }
    /// 清空某用户所有 token（通过集合 smembers）
    async fn clear_tokens_by_user(&self, agent_id: &str, user_id: &UserId) -> Result<()> {
        let token_set_key = format!("token:uid:{}:{}", agent_id, user_id);
        let mut conn = self.pool.get().await?;
        let tokens: Vec<String> = conn.smembers(&token_set_key).await.unwrap_or_default();
        for token in tokens {
            self.delete_token(&token).await?;
        }
        let _: () = conn.del(&token_set_key).await?;
        Ok(())
    }
    /// 检查 token 是否存在
    async fn verify_token(&self, token: &str) -> Result<bool> {
        let mut conn = self.pool.get().await?;
        let exists: bool = conn
            .exists(format!("token:{}", token))
            .await
            .context("检查 token 失败")?;
        Ok(exists)
    }

    /// 获取 token 对应的客户端身份
    async fn get_client_token(&self, token: &str) -> Result<ClientTokenDto> {
        let mut conn = self.pool.get().await?;
        let json: String = conn
            .get(format!("token:{}", token))
            .await
            .context("获取 token 数据失败")?;
        serde_json::from_str(&json).context("反序列化 ClientTokenDto 失败")
    }

    /// 通过 token 查找用户完整信息
    async fn find_user_by_token(&self, token: &str) -> Result<Option<ClientInfo>> {
        let dto = self.get_client_token(token).await?;
        let user = self
            .get_user_info(&dto.agent_id, &dto.uid)
            .await?
            .ok_or_else(|| AppError::BizError("用户不存在".to_string()))?;
        Ok(Some(user))
    }

    async fn add_friend(
        &self,
        agent_id: &str,
        user_id: &UserId,
        friend_id: &UserId,
        nickname: Option<String>,
        source_type: &FriendSourceType,
        remark: Option<String>,
    ) -> Result<()> {
        // ---------- 1. 验证用户和好友存在 ----------
        let user_manager = UserManager::get();
        let (client_opt, friend_opt) = try_join!(
            user_manager.get_user_info(agent_id, user_id),
            user_manager.get_user_info(agent_id, friend_id)
        )?;

        let client_info = client_opt.ok_or_else(|| anyhow!("用户 {} 不存在", user_id))?;
        let friend_info = friend_opt.ok_or_else(|| anyhow!("好友 {} 不存在", friend_id))?;

        let nickname_to_friend = if nickname.is_some() {
            nickname
        } else {
            Some(friend_info.name)
        };
        let nickname_to_user = Some(client_info.name.as_str());

        // ---------- 3. Redis 写入 ----------
        let redis_key = |uid: &UserId| format!("friend:user:{}:{}", agent_id, uid);
        let mut conn = self.pool.get().await?;
        let _: () = conn.sadd(redis_key(user_id), friend_id).await?;
        let _: () = conn.sadd(redis_key(friend_id), user_id).await?;

        // ---------- 4. 本地缓存写入（可选） ----------
        if self.use_local_cache {
            let cache_key = |uid: &UserId| format!("{}:{}", agent_id, uid);
            let insert_cache = |key: String, id: &UserId| {
                self.friend_map
                    .entry(key)
                    .or_insert_with(DashMap::new)
                    .insert(id.clone(), ());
            };
            insert_cache(cache_key(user_id), friend_id);
            insert_cache(cache_key(friend_id), user_id);
        }

        // ---------- 5. 数据库持久化 ----------
        let friend_service = UserFriendService::get();
        friend_service
            .add_friend(
                agent_id,
                user_id,
                friend_id,
                nickname_to_friend.as_deref(),
                source_type,
                remark.as_deref(),
            )
            .await?;
        friend_service
            .add_friend(
                agent_id,
                friend_id,
                user_id,
                nickname_to_user,
                source_type,
                remark.as_deref(),
            )
            .await?;

        // ---------- 6. Kafka 消息通知 ----------
        let kafka_service = KafkaService::get();
        let app_config = AppConfig::get();
        let topic = &app_config.kafka.topic_single;
        let time = now();

        // 构造好友事件
        let make_event = |from_uid: &str, to_uid: &str| FriendEventMsg {
            message_id: Some(build_snow_id()),
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
            let node_index = 0 as u8;
            let message_id = &event.message_id.clone().unwrap().to_string();
            if let Err(e) = kafka_service
                .send_proto(
                    &ByteMessageType::FriendEventMsgType,
                    &node_index,
                    &event,
                    &message_id,
                    topic,
                )
                .await
            {
                //发送失败时把记录插入数据库
            }
        }
        Ok(())
    }

    async fn remove_friend(
        &self,
        agent_id: &str,
        user_id: &UserId,
        friend_id: &UserId,
    ) -> Result<()> {
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
        friend_service
            .remove_friend(agent_id, user_id, friend_id)
            .await?;
        friend_service
            .remove_friend(agent_id, friend_id, user_id)
            .await?;

        // ---------- 4. 发送 Kafka 通知（可选） ----------
        let kafka_service = KafkaService::get();
        let app_config = AppConfig::get();
        let topic = &app_config.kafka.topic_single;
        let time = now();

        let make_event = |from_uid: &UserId, to_uid: &UserId| FriendEventMsg {
            message_id: Some(build_snow_id()), // 为删除事件生成唯一 ID
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
            let node_index = 0 as u8;
            let message_id = &event.message_id.clone().unwrap().to_string();
            if let Err(e) = kafka_service
                .send_proto(
                    &ByteMessageType::FriendEventMsgType,
                    &node_index,
                    &event,
                    &message_id,
                    topic,
                )
                .await
            {
                log::warn!("Kafka 消息发送失败 [{}]: {:?}", message_id, e);
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
        let exists: bool = conn
            .sismember(&redis_key, friend_id)
            .await
            .context("Redis SISMEMBER 查询失败")?;

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
        let exists_in_db = friend_service
            .dao
            .find_one(filter)
            .await
            .map(|opt| opt.is_some())
            .unwrap_or(false);
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
        let redis_friends: Vec<String> = conn
            .smembers(&redis_key)
            .await
            .context("Redis SMEMBERS 获取好友列表失败")?;

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
        let mongo_friends = friend_service
            .dao
            .query(filter)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|f| f.friend_id)
            .collect::<Vec<UserId>>();

        // 同步回 Redis 和本地缓存（可选）
        if !mongo_friends.is_empty() {
            let _: () = conn
                .sadd(&redis_key, &mongo_friends)
                .await
                .unwrap_or_default();

            if self.use_local_cache {
                let map = self
                    .friend_map
                    .entry(cache_key.clone())
                    .or_insert_with(DashMap::new);
                for fid in &mongo_friends {
                    map.insert(fid.clone(), ());
                }
            }
        }

        Ok(mongo_friends)
    }

    async fn friend_block(
        &self,
        agent_id: &str,
        user_id: &UserId,
        friend_id: &UserId,
    ) -> Result<()> {
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
            friend_service
                .get_friend_detail(agent_id, user_id, friend_id)
                .await?
        } else {
            None
        };

        // 三层都找不到关系才报错
        if !is_friend && mongo_friend.is_none() {
            return Err(anyhow!(
                "用户 {} 与 {} 非好友关系，无法拉黑",
                user_id,
                friend_id
            ));
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
        friend_service
            .dao
            .up_property(&friend_info.id, "is_blocked", true)
            .await?;
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
        let make_event = |from_uid: &str, to_uid: &str| FriendEventMsg {
            message_id: Some(build_snow_id()),
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
            let node_index = 0 as u8;
            let message_id = &event.message_id.clone().unwrap().to_string();
            if let Err(e) = kafka_service
                .send_proto(
                    &ByteMessageType::FriendEventMsgType,
                    &node_index,
                    &event,
                    &message_id,
                    topic,
                )
                .await
            {
                log::warn!("Kafka 消息发送失败 [{}]: {:?}", message_id, e);
            }
        }
        Ok(())
    }

    async fn friend_unblock(
        &self,
        agent_id: &str,
        user_id: &UserId,
        friend_id: &UserId,
    ) -> Result<()> {
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
            friend_service
                .get_friend_detail(agent_id, user_id, friend_id)
                .await?
        } else {
            None
        };

        // 三层都找不到关系才报错
        if !is_friend && mongo_friend.is_none() {
            return Err(anyhow!(
                "用户 {} 与 {} 非好友关系，无法拉黑",
                user_id,
                friend_id
            ));
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
        friend_service
            .dao
            .up_property(&friend_info.id, "is_blocked", false)
            .await?;
        // ---------- 3. Redis 删除黑名单集合 ----------
        let redis_block_key = format!("block:user:{}:{}", agent_id, user_id);
        let _: () = conn.srem(&redis_block_key, friend_id).await?;
        // ---------- 4. 可选：发送 Kafka 取消拉黑事件 ----------
        let time = now();
        let message_id = build_snow_id();
        // 构造好友事件
        let make_event = |from_uid: &str, to_uid: &str| FriendEventMsg {
            message_id: Some(message_id.clone()),
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
            let node_index = 0 as u8;
            if let Err(e) = kafka_service
                .send_proto(
                    &ByteMessageType::FriendEventMsgType,
                    &node_index,
                    &event,
                    &message_id.to_string(),
                    topic,
                )
                .await
            {
                log::warn!("Kafka 消息发送失败 [{}]: {:?}", message_id, e);
            }
        }
        Ok(())
    }
}
