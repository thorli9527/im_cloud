use crate::entitys::user_entity::UserInfoEntity;
use anyhow::Result;
use anyhow::anyhow;
use common::config::AppConfig;
use common::redis::redis_template::RedisTemplate;
use common::redis::redis_template::ValueOps;
use common::repository_util::{BaseRepository, Repository};
use common::util::common_utils::{build_md5_with_key, build_uuid};
use common::util::date_util::now;
use mongodb::Database;
use mongodb::bson::doc;
use once_cell::sync::OnceCell;
use std::sync::Arc;

#[derive(Debug)]
pub struct UserService {
    pub dao: BaseRepository<UserInfoEntity>,
}

impl UserService {
    pub fn new(db: Database) -> Self {
        let collection = db.collection("user_info");
        Self { dao: BaseRepository::new(db, collection.clone()) }
    }
    pub fn init(db: Database) {
        let instance = Self::new(db);
        INSTANCE.set(Arc::new(instance)).expect("INSTANCE already initialized");
    }

    /// 获取单例
    pub fn get() -> Arc<Self> {
        INSTANCE.get().expect("INSTANCE is not initialized").clone()
    }

    pub async fn login_out(&self, token: &str) -> Result<()> {
        let redis_template = RedisTemplate::get();
        let token_key = format!("token:{}", token);
        redis_template.ops_for_value().delete(&token_key).await.expect("删除token失败");
        Ok(())
    }
    pub async fn build_login(
        &self,
        user_name: &str,
        password: &str,
    ) -> Result<(String, UserInfoEntity)> {
        let user_info = self.dao.find_one(doc! {"user_name": user_name}).await?;
        if user_info.is_none() {
            return Err(anyhow!("用户不存在"));
        }

        let sys_config = AppConfig::get().clone().sys.clone().unwrap();
        let md5_key = &sys_config.md5_key.unwrap();
        let user_info = user_info.unwrap();

        if user_info.password != build_md5_with_key(password, md5_key) {
            return Err(anyhow!("密码错误"));
        }

        let redis_template = RedisTemplate::get();
        let token = build_uuid();
        let token_key = format!("token:{}", token);
        redis_template
            .ops_for_value()
            .set(&token_key, &user_info, Some(30))
            .await
            .expect("redis.error");

        Ok((token, user_info))
    }

    /// 根据用户名查询用户信息
    pub async fn get_by_username(&self, username: &str) -> Result<Option<UserInfoEntity>> {
        let filter = doc! { "user_name": username };
        let mut users = self.dao.query(filter).await?;
        if users.is_empty() { Ok(None) } else { Ok(Some(users.remove(0))) }
    }

    /// 验证用户名和明文密码，匹配则返回用户信息，否则返回None
    pub async fn authenticate(
        &self,
        username: &str,
        plain_password: &str,
    ) -> Result<Option<(String, UserInfoEntity)>> {
        if let Some(user) = self.get_by_username(username).await? {
            // 检查用户状态
            if !user.status {
                return Ok(None); // 用户被禁用
            }
            let sys_config = AppConfig::get().clone().sys.clone().unwrap();
            let md5_key = &sys_config.md5_key.unwrap();

            if user.password != build_md5_with_key(plain_password, md5_key) {
                let redis_template = RedisTemplate::get();
                let token = build_uuid();
                let token_key = format!("token:{}", token);
                redis_template
                    .ops_for_value()
                    .set(&token_key, &user, Some(30))
                    .await
                    .expect("redis.error");
                Ok(Some((token, user.clone())))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }
    /// 添加新用户（注册功能，密码在外部先加密后传入）
    pub async fn add_user(
        &self,
        username: &str,
        hashed_password: &str,
        is_admin: bool,
        status: bool,
    ) -> Result<String> {
        let entity = UserInfoEntity {
            id: "".into(),
            user_name: username.into(),
            password: hashed_password.into(),
            status,
            is_admin,
            create_time: now() as u64,
            update_time: now() as u64,
        };
        self.dao.insert(&entity).await
    }
}
static INSTANCE: OnceCell<Arc<UserService>> = OnceCell::new();
