use crate::biz_service::client_service::ClientService;
use crate::biz_service::friend_service::UserFriendService;
use crate::biz_service::kafka_socket_service::KafkaService;
use crate::manager::user_manager_core::{UserManager, UserManagerOpt, USER_ONLINE_TTL_SECS};

use crate::protocol::common::{ByteMessageType, ClientEntity};
use crate::protocol::msg::auth::{DeviceType, LoginRespMsg, LogoutRespMsg, OfflineStatueMsg, OnlineStatusMsg};
use crate::protocol::msg::friend::{EventStatus, FriendEventMsg, FriendEventType, FriendSourceType};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use common::config::AppConfig;
use common::errors::AppError;
use common::repository_util::Repository;
use common::util::common_utils::{build_md5_with_key, build_snow_id, build_uuid};
use common::util::date_util::now;
use common::{ClientTokenDto, UserId};
use deadpool_redis::redis::AsyncCommands;
use mongodb::bson::doc;
use tokio::try_join;

pub const TOKEN_EXPIRE_SECS: u64 = 60 * 60 * 24 * 7;
#[async_trait]
impl UserManagerOpt for UserManager {

    async fn login(
        &self,
        message_id: &u64,
        user_name: &str,
        password: &str,
        device_type: &DeviceType,
    ) -> anyhow::Result<String> {
        let client_info = self.get_user_info_by_name( user_name).await?;

        if client_info.is_none() {
            return Err(anyhow::anyhow!("user.or.password.error"));
        }
        let app_config = AppConfig::get();

        let string = build_md5_with_key(password, &app_config.get_sys().md5_key.unwrap());
        let client = client_info.unwrap();
        if &client.password != &string {
            return Err(anyhow::anyhow!("user.or.password.error"));
        }
        let user_id = client.id.clone() as UserId;
        self.online( &user_id, device_type).await?;

        let token = self.build_token( &user_id, device_type).await?;

        let kafka_service = KafkaService::get();
        // 发送登录响应消息
        let login_msg = &LoginRespMsg {
            message_id: message_id.clone(),
            token: token.clone(),
            expires_at: now() as u64,
        };
        kafka_service
            .send_proto(
                &ByteMessageType::LoginRespMsgType,
                login_msg,
                &login_msg.message_id,
                &AppConfig::get().get_kafka().topic_group,
            )
            .await?;
        Ok(token)
    }

    async fn logout(
        &self,
        message_id: &u64,
        uid: &UserId,
        device_type: &DeviceType,
    ) -> Result<()> {
        self.offline( uid, device_type).await?;
        let kafka_service = KafkaService::get();
        let token = self
            .get_token_by_uid_device(uid, device_type)
            .await?;
        if let Some(token) = token {
            self.delete_token(&token).await?;
        }
        // 发送登出响应消息
        let log_out_msg = LogoutRespMsg {
            message_id: message_id.clone(),
        };
        kafka_service
            .send_proto(
                &ByteMessageType::LogoutRespMsgType,
                &log_out_msg,
                &log_out_msg.message_id,
                &AppConfig::get().get_kafka().topic_group,
            )
            .await?;
        Ok(())
    }
    async fn online(&self,  user_id: &UserId, device_type: &DeviceType) -> Result<()> {
        let (redis_key, field_key) = self.make_online_key( user_id);
        let mut conn = self.pool.get().await?;

        // 获取现有设备
        let existing: Option<String> = conn.hget(&redis_key, &field_key).await?;
        let mut devices = match existing {
            Some(s) => s.split(',').map(|s| s.to_string()).collect::<std::collections::HashSet<_>>(),
            None => std::collections::HashSet::new(),
        };

        let device_str = (*device_type as u8).to_string();
        let is_first_online = devices.is_empty();
        devices.insert(device_str.clone());

        let new_value = devices.into_iter().collect::<Vec<_>>().join(",");
        let _: () = conn.hset(&redis_key, &field_key, &new_value).await?;
        let _: () = conn.expire(&redis_key, USER_ONLINE_TTL_SECS as i64).await?;

        if is_first_online {
            let kafka = KafkaService::get();
            let online_msg = OnlineStatusMsg {
                message_id: build_snow_id(),
                uid: user_id.clone(),
                device_type: *device_type as i32,
                client_id: "".to_string(),
                login_time: now(),
            };
            kafka.send_proto(
                &ByteMessageType::LogoutRespMsgType,
                &online_msg,
                &online_msg.message_id,
                &AppConfig::get().get_kafka().topic_group,
            ).await?;
        }

        Ok(())
    }

    async fn is_online(&self, user_id: &UserId) -> Result<bool> {
        let (redis_key, field_key) = self.make_online_key(user_id);
        let mut conn = self.pool.get().await?;
        Ok(conn.hexists(&redis_key, &field_key).await?)
    }

    async fn is_all_device_offline(&self, user_id: &UserId) -> Result<bool> {
        Ok(!self.is_online(user_id).await?)
    }

    async fn get_online_devices(&self,  user_id: &UserId) -> Result<Vec<DeviceType>> {
        let (redis_key, field_key) = self.make_online_key( user_id);
        let mut conn = self.pool.get().await?;
        if let Some(s) = conn.hget::<_, _, Option<String>>(&redis_key, &field_key).await? {
            Ok(s.split(',')
                .filter_map(|d| d.parse::<i32>().ok())
                .filter_map(DeviceType::from_i32)
                .collect())
        } else {
            Ok(vec![])
        }
    }

    async fn offline(&self,  user_id: &UserId, device_type: &DeviceType) -> Result<()> {
        let (redis_key, field_key) = self.make_online_key( user_id);
        let mut conn = self.pool.get().await?;

        let existing: Option<String> = conn.hget(&redis_key, &field_key).await?;
        if let Some(s) = existing {
            let mut devices: std::collections::HashSet<_> = s.split(',').map(|x| x.to_string()).collect();
            devices.remove(&(*device_type as u8).to_string());

            if devices.is_empty() {
                let _: () = conn.hdel(&redis_key, &field_key).await?;
            } else {
                let new_val = devices.into_iter().collect::<Vec<_>>().join(",");
                let _: () = conn.hset(&redis_key, &field_key, &new_val).await?;
            }
        }

        let is_all_offline = self.is_all_device_offline(user_id).await?;
        if is_all_offline {
            let kafka = KafkaService::get();
            let offline_msg = OfflineStatueMsg {
                message_id: build_snow_id(),
                uid: user_id.clone(),
                device_type: DeviceType::All as i32,
                client_id: "".to_string(),
                logout_time: now(),
                reason: "".to_string(),
            };
            kafka.send_proto(
                &ByteMessageType::LogoutRespMsgType,
                &offline_msg,
                &offline_msg.message_id,
                &AppConfig::get().get_kafka().topic_group,
            ).await?;
        }

        Ok(())
    }

    async fn sync_user(&self, _user: ClientEntity) -> anyhow::Result<()> {
        let user_id = &_user.uid;
        let mut conn = self.pool.get().await?;
        let user_info_json = serde_json::to_string(&_user)?;
        let string = format!(":client:{}",  user_id);
        let _: () = conn.set(string, user_info_json).await?;
        Ok(())
    }

    async fn remove_user(&self,  user_id: &UserId) -> Result<()> {
        let mut conn = self.pool.get().await?;
        let redis_online_key = format!("online:user:agent:{}",user_id);
        let _: () = conn
            .del(redis_online_key)
            .await
            .context("删除在线状态失败")?;
        let key = format!("client:{}", user_id);
        let _: () = conn.del(&key).await.context("删除用户缓存失败")?;
        // 发消息 册除用户
        Ok(())
    }

    /// 优先查 Redis，否则从 MongoDB 兜底并写入 Redis
    async fn get_user_info(&self, user_id: &UserId) -> Result<Option<ClientEntity>> {
        let mut conn = self.pool.get().await?;
        let key = format!("client:{}",  user_id);

        // 1. 尝试从 Redis 获取
        if let Ok(Some(cached_json)) = conn.get::<_, Option<String>>(&key).await {
            let parsed = serde_json::from_str(&cached_json)?;
            return Ok(Some(parsed));
        }

        // 2. Redis 未命中，从 MongoDB 查找
        let client_service = ClientService::get();
        let client_opt = client_service
            .dao
            .find_one(doc! {  "userId": user_id })
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
        name: &str,
    ) -> Result<Option<ClientEntity>> {
        let client_service = ClientService::get();
        let client_opt = client_service
            .dao
            .find_one(doc! {  "username": name })
            .await?;
        return Ok(client_opt);
    }

    /// 构建 Token：写入主数据、索引、集合（反向查找用）
    async fn build_token(
        &self,
        uid: &UserId,
        device_type: &DeviceType,
    ) -> Result<String> {
        let token_key = build_uuid();
        let token_data_key = format!("token:{}", token_key);
        let token_index_key = format!("token:index:{}:{}", uid, *device_type as i32);
        let token_set_key = format!("token:uid:{}",  uid);

        let dto = ClientTokenDto {
            uid: uid.clone(),
            device_type: *device_type as u8,
        };

        let token_str = serde_json::to_string(&dto).context("序列化 ClientTokenDto 失败")?;
        let mut conn = self.pool.get().await?;

        // 主数据 & 单设备索引
        let _: () = conn
            .set_ex(&token_data_key, token_str, TOKEN_EXPIRE_SECS)
            .await?;
        let _: () = conn
            .set_ex(&token_index_key, &token_key, TOKEN_EXPIRE_SECS)
            .await?;

        // 添加到 uid 所有 token 集合（便于注销所有 token）
        let _: () = conn.sadd(&token_set_key, &token_key).await?;
        let _: () = conn
            .expire(&token_set_key, TOKEN_EXPIRE_SECS as i64)
            .await?;

        Ok(token_key)
    }
    /// 删除指定 token（包括索引 + 主数据 + 集合成员）
    async fn delete_token(&self, token: &str) -> Result<()> {
        let mut conn = self.pool.get().await?;
        let token_key = format!("token:{}", token);

        if let Ok(json) = conn.get::<_, String>(&token_key).await {
            if let Ok(dto) = serde_json::from_str::<ClientTokenDto>(&json) {
                let index_key = format!(
                    "token:index:{}:{}",
                 dto.uid, dto.device_type
                );
                let token_set_key = format!("token:uid:{}",  dto.uid);

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
    /// 检查 token 是否存在
    async fn verify_token(&self, token: &str) -> Result<bool> {
        let mut conn = self.pool.get().await?;
        let exists: bool = conn
            .exists(format!("token:{}", token))
            .await
            .context("检查 token 失败")?;
        Ok(exists)
    }
    /// 清空某用户所有 token（通过集合 smembers）
    async fn clear_tokens_by_user(&self, user_id: &UserId) -> Result<()> {
        let token_set_key = format!("token:uid:{}", user_id);
        let mut conn = self.pool.get().await?;
        let tokens: Vec<String> = conn.smembers(&token_set_key).await.unwrap_or_default();
        for token in tokens {
            self.delete_token(&token).await?;
        }
        let _: () = conn.del(&token_set_key).await?;
        Ok(())
    }
    /// 获取指定 uid + 设备的 token（单设备支持）
    async fn get_token_by_uid_device(
        &self,
        uid: &UserId,
        device_type: &DeviceType,
    ) -> Result<Option<String>> {
        let mut conn = self.pool.get().await?;
        let index_key = format!("token:index:{}:{}", uid, *device_type as i32);
        let token: Option<String> = conn.get(&index_key).await.context("获取 token 索引失败")?;
        Ok(token)
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
    async fn find_user_by_token(&self, token: &str) -> Result<Option<ClientEntity>> {
        let dto = self.get_client_token(token).await?;
        let user = self
            .get_user_info( &dto.uid)
            .await?
            .ok_or_else(|| AppError::BizError("用户不存在".to_string()))?;
        Ok(Some(user))
    }
    async fn is_friend(&self,uid: &UserId, friend_id: &UserId) -> Result<bool> {

        // 1. Redis 查询
        let redis_key = format!("friend:user:{}", uid);
        let mut conn = self.pool.get().await?;
        let exists: bool = conn
            .sismember(&redis_key, friend_id)
            .await
            .context("Redis SISMEMBER 查询失败")?;

        if exists {
            return Ok(true);
        }

        // 2. MongoDB 兜底
        let filter = doc! {
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

    async fn get_friends(&self,  user_id: &UserId) -> Result<Vec<UserId>> {
        let cache_key = format!("{}", user_id);

        // 1. Redis 查询
        let redis_key = format!("friend:user:{}",  user_id);
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
        // 2. MongoDB 兜底查询
        let filter = doc! {
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
        }
        Ok(mongo_friends)
    }

    async fn friend_block(
        &self,
        user_id: &UserId,
        friend_id: &UserId,
    ) -> Result<()> {
        let friend_service = UserFriendService::get();

        // ---------- 1. 校验是否有好友关系 ----------
        let mut is_friend = false;

        let mut conn = self.pool.get().await?;

        // Redis 判断
        if !is_friend {
            let redis_key = format!("friend:user:{}",  user_id);
            let exists: bool = conn.sismember(&redis_key, friend_id).await.unwrap_or(false);
            if exists {
                is_friend = true;
            }
        }

        // Mongo 判断
        let mongo_friend = if !is_friend {
            // 此处也顺便获取记录用于更新
            friend_service
                .get_friend_detail( user_id, friend_id)
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
            let redis_block_key = format!("block:user:{}",  user_id);
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
        let redis_block_key = format!("block:user:{}",  user_id);
        let _: () = conn.sadd(&redis_block_key, friend_id).await?;
        // ---------- 4. 可选：发送 Kafka 拉黑事件 ----------
        // ---------- 6. Kafka 消息通知 ----------
        let kafka_service = KafkaService::get();
        let app_config = AppConfig::get();
        let topic = &app_config.get_kafka().topic_single;
        let time = now() as u64;

        // 构造好友事件
        let make_event = |from_uid: &str, to_uid: &str| FriendEventMsg {
            message_id: build_snow_id(),
            from_uid: from_uid.to_string(),
            to_uid: to_uid.to_string(),
            event_type: FriendEventType::FriendBlock as i32,
            message: String::new(),
            status: EventStatus::Done as i32,
            source_type: FriendSourceType::FriendSourceSystem as i32,
            from_a_name: "".to_string(),
            to_a_name: "".to_string(),
            from_remark: None,
            created_at: time,
            updated_at: time,
            to_remark: None,
        };

        // 同步通知
        for (form_uid, to_uid) in [(user_id, &friend_id)] {
            let event = make_event(form_uid, to_uid);
            let message_id = &event.message_id.clone();
            if let Err(e) = kafka_service
                .send_proto(
                    &ByteMessageType::FriendEventMsgType,
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
        user_id: &UserId,
        friend_id: &UserId,
    ) -> Result<()> {
        let friend_service = UserFriendService::get();

        // ---------- 1. 校验是否有好友关系 ----------
        let mut is_friend = false;

        let mut conn = self.pool.get().await?;

        // Redis 判断
        if !is_friend {
            let redis_key = format!("friend:user:{}", user_id);
            let exists: bool = conn.sismember(&redis_key, friend_id).await.unwrap_or(false);
            if exists {
                is_friend = true;
            }
        }

        // Mongo 判断
        let mongo_friend = if !is_friend {
            // 此处也顺便获取记录用于更新
            friend_service
                .get_friend_detail( user_id, friend_id)
                .await?
        } else {
            None
        };

        // 2层都找不到关系才报错
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
            let redis_block_key = format!("block:user:{}",  user_id);
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
        let redis_block_key = format!("block:user:{}", user_id);
        let _: () = conn.srem(&redis_block_key, friend_id).await?;
        // ---------- 4. 可选：发送 Kafka 取消拉黑事件 ----------
        let time = now() as u64;
        let message_id = build_snow_id();
        // 构造好友事件
        let make_event = |from_uid: &str, to_uid: &str| FriendEventMsg {
            message_id: message_id.clone(),
            from_uid: from_uid.to_string(),
            to_uid: to_uid.to_string(),
            event_type: FriendEventType::FriendUnblock as i32,
            message: String::new(),
            status: EventStatus::Done as i32,
            source_type: FriendSourceType::FriendSourceSystem as i32,
            from_a_name: "".to_string(),
            to_a_name: "".to_string(),
            from_remark: None,
            created_at: time,
            updated_at: time,
            to_remark: None,
        };
        // ---------- 6. Kafka 消息通知 ----------
        let kafka_service = KafkaService::get();
        let app_config = AppConfig::get();
        let topic = &app_config.get_kafka().topic_single;
        // 同步通知双方
        for (form_uid, to_uid) in [(user_id, &friend_id)] {
            let event = make_event(form_uid, to_uid);
            if let Err(e) = kafka_service
                .send_proto(
                    &ByteMessageType::FriendEventMsgType,
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
}
