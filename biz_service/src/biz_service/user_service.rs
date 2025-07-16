use crate::entitys::user_entity::UserInfoEntity;
use anyhow::Result;
use anyhow::anyhow;
use common::config::AppConfig;
use common::redis::redis_template::RedisTemplate;
use common::redis::redis_template::ValueOps;
use common::repository_util::{BaseRepository, Repository};
use common::util::common_utils::{build_md5_with_key, build_uuid};
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
        Self {
            dao: BaseRepository::new(db, collection.clone()),
        }
    }
    pub fn init(db: Database) {
        let instance = Self::new(db);
        INSTANCE
            .set(Arc::new(instance))
            .expect("INSTANCE already initialized");
    }

    /// 获取单例
    pub fn get() -> Arc<Self> {
        INSTANCE.get().expect("INSTANCE is not initialized").clone()
    }

    pub async fn login_out(&self, token: &str) -> Result<()> {
        let redis_template = RedisTemplate::get();
        let token_key = format!("token:{}", token);
        redis_template
            .ops_for_value()
            .delete(&token_key)
            .await
            .expect("删除token失败");
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
}
static INSTANCE: OnceCell<Arc<UserService>> = OnceCell::new();
