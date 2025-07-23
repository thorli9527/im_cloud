use crate::biz_service::client_service::ClientService;
use crate::biz_service::kafka_socket_service::KafkaService;
use crate::manager::user_manager::{UserManager, UserManagerOpt};
use crate::manager::user_manager_auth::{
    ResetPasswordType, UserManagerAuth, UserManagerAuthOpt, UserRegType,
};
use crate::protocol::common::{ByteMessageType, ClientEntity};
use crate::protocol::msg::auth::{DeviceType, LoginRespMsg, LogoutRespMsg};
use anyhow::anyhow;
use async_trait::async_trait;
use bson::doc;
use common::config::AppConfig;
use common::repository_util::Repository;
use common::util::common_utils::{build_md5_with_key, build_snow_id, build_uid, build_uuid};
use common::util::date_util::now;
use common::UserId;
use deadpool_redis::redis::AsyncCommands;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct VerifySession {
    user_name: String,
    code: String,
    reg_type: u8,
    target: String,
}
#[async_trait]
impl UserManagerAuthOpt for UserManagerAuth {
    async fn login(
        &self,
        message_id: &u64,
        user_name: &str,
        password: &str,
        device_type: &DeviceType,
    ) -> anyhow::Result<String> {
        let user_manager = UserManager::get();
        let client_info = user_manager.get_user_info_by_name(user_name).await?;

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
        user_manager.online(&user_id, device_type).await?;

        let token = user_manager.build_token(&user_id, device_type).await?;

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
    ) -> anyhow::Result<()> {
        let user_manager = UserManager::get();
        user_manager.offline(uid, device_type).await?;
        let kafka_service = KafkaService::get();
        let token = user_manager.get_token_by_uid_device(uid, device_type).await?;
        if let Some(token) = token {
            user_manager.delete_token(&token).await?;
        }
        // 发送登出响应消息
        let log_out_msg = LogoutRespMsg { message_id: message_id.clone() };
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

    async fn register(
        &self,
        user_name: &str,
        password: &str,
        reg_type: &UserRegType,
        target: &str,
    ) -> anyhow::Result<String> {
        let client_service = ClientService::get();
        let existed = client_service.dao.find_one(doc! { "name": user_name }).await?.is_some();

        if existed {
            return Err(anyhow!("用户名已存在"));
        }

        // 生成验证码
        let code = format!("{:06}", rand::random::<u32>() % 1000000);
        let uuid = build_uuid();

        let redis_key = format!("register:verify:uuid:{}", uuid);
        let session = VerifySession {
            user_name: user_name.to_string(),
            code: code.clone(),
            reg_type: *reg_type as u8,
            target: target.to_string(),
        };
        let json_data = serde_json::to_string(&session)?;

        let mut conn = UserManager::get().pool.get().await?;
        conn.set_ex::<_, _, ()>(&redis_key, json_data, 300).await?;

        // 实际生产应调用短信/邮箱服务
        log::info!("[注册验证码] 发送到 {}: {}，uuid={}", target, code, uuid);

        // 返回 uuid 给前端
        Ok(uuid)
    }

    async fn register_verify_code(
        &self,
        user_name: &str,
        password: &str,
        reg_id: &str,
        code: &str,
        reg_type: &UserRegType,
    ) -> anyhow::Result<String> {
        // 构造 Redis Key
        let redis_key = format!("register:verify:uuid:{}", reg_id);
        let mut conn = UserManager::get().pool.get().await?;

        // Step 1: 获取 Redis 中缓存的验证码信息
        let cached: Option<String> = conn.get(&redis_key).await?;
        let Some(json_str) = cached else {
            return Err(anyhow!("验证码已过期或无效"));
        };

        let session: VerifySession = serde_json::from_str(&json_str)?;
        if session.code != code {
            return Err(anyhow!("验证码错误"));
        }

        if session.user_name != user_name {
            return Err(anyhow!("用户名与验证码不一致"));
        }

        if session.reg_type != *reg_type as u8 {
            return Err(anyhow!("注册方式与验证码不一致"));
        }

        // Step 2: 检查用户名是否已存在
        let client_service = ClientService::get();
        let existed = client_service.dao.find_one(doc! { "name": user_name }).await?.is_some();
        if existed {
            return Err(anyhow!("用户名已存在"));
        }

        // Step 3: 创建用户
        let uid = build_uid();
        let hashed_pwd =
            build_md5_with_key(password, &AppConfig::get().get_sys().md5_key.clone().unwrap());
        let now = now() as u64;

        let client = ClientEntity {
            id: "".to_string(),
            uid: uid.clone(),
            name: user_name.to_string(),
            password: hashed_pwd,
            email: if matches!(reg_type, UserRegType::Email) {
                Some(session.target.clone())
            } else {
                None
            },
            phone: if matches!(reg_type, UserRegType::Phone) {
                Some(session.target.clone())
            } else {
                None
            },
            language: None,
            avatar: "".to_string(),
            allow_add_friend: 0,
            gender: 0,
            user_type: 0,
            profile_fields: Default::default(),
            create_time: now,
            update_time: now,
        };

        client_service.dao.insert(&client).await?;

        // Step 4: 删除验证码缓存
        let _: () = conn.del(&redis_key).await?;

        // Step 5: 返回 UID
        Ok(uid)
    }

    async fn change_password(
        &self,
        token: &str,
        old_password: &str,
        new_password: &str,
    ) -> anyhow::Result<()> {
        let user_manager = UserManager::get();
        let dto = user_manager.get_client_token(token).await?;
        let client =
            user_manager.get_user_info(&dto.uid).await?.ok_or_else(|| anyhow!("用户不存在"))?;
        let app_config = AppConfig::get();
        let hashed_old =
            build_md5_with_key(old_password, &app_config.get_sys().md5_key.clone().unwrap());

        if client.password != hashed_old {
            return Err(anyhow!("原密码错误"));
        }

        let new_pwd =
            build_md5_with_key(new_password, &app_config.get_sys().md5_key.clone().unwrap());
        ClientService::get().dao.up_property(&client.id, "password", new_pwd).await?;
        Ok(())
    }

    async fn reset_password_build_code(
        &self,
        reset_type: &ResetPasswordType,
        user_name: &str,
    ) -> anyhow::Result<()> {
        let redis_key = format!("reset:verify:{}:{}", *reset_type as u8, user_name);
        let code = format!("{:06}", rand::random::<u32>() % 1000000);
        let mut conn = UserManager::get().pool.get().await?;
        conn.set_ex::<_, _, ()>(&redis_key, code.clone(), 300).await?;
        log::info!("[重置密码验证码] 发送到 {}: {}", user_name, code);
        Ok(())
    }

    async fn reset_password_verify_code(
        &self,
        reset_type: &ResetPasswordType,
        user_name: &str,
        code: &str,
        new_password: &str,
    ) -> anyhow::Result<()> {
        let redis_key = format!("reset:verify:{}:{}", *reset_type as u8, user_name);
        let mut conn = UserManager::get().pool.get().await?;
        let cached: Option<String> = conn.get(&redis_key).await?;
        if cached.as_deref() != Some(code) {
            return Err(anyhow!("验证码错误或已过期"));
        }

        let filter = if matches!(reset_type, ResetPasswordType::Phone) {
            doc! {"phone": user_name}
        } else {
            doc! {"email": user_name}
        };

        let new_pwd =
            build_md5_with_key(new_password, &AppConfig::get().get_sys().md5_key.clone().unwrap());
        let client_service = ClientService::get();
        let client_opt = client_service.dao.find_one(filter).await?;
        if let Some(client) = client_opt {
            client_service.dao.up_property(&client.id, "password", new_pwd).await?;
            let _: () = conn.del(&redis_key).await?;
            return Ok(());
        }
        Err(anyhow!("用户不存在"))
    }
}
