use crate::biz_service::cache_service::get_agent_cache;
use crate::entitys::agent_entity::AgentInfo;
use common::errors::AppError;
use common::repository_util::{BaseRepository, Repository};
use mongodb::bson::doc;
use mongodb::Database;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::sync::Arc;
use utoipa::ToSchema;


#[derive(Debug)]
pub struct AgentService {
    pub dao: BaseRepository<AgentInfo>,
}

impl AgentService {

    pub fn new(db: Database) -> Self {
        let collection = db.collection("agent_info");
        let service = Self { dao: BaseRepository::new(db, collection.clone()) };
        service
    }

    pub async fn find_by_app_key(&self, app_key: impl AsRef<str> + Clone) -> Result<AgentInfo, AppError> {
        let cache = get_agent_cache();
        if let Some(agent) = cache.get(app_key.as_ref()) {
            return Ok(agent);
        }

        let key = app_key.as_ref();
        let agent = self.dao.find_one(doc! { "app_key": key }).await?.unwrap();
        cache.insert(agent.app_key.clone(), agent.clone());
        Ok(agent)
    }

    //生成hash签名算法
    pub fn generate_checksum(&self, app_secret: impl AsRef<str> + std::fmt::Display, nonce: impl AsRef<str> + std::fmt::Display, cur_time: i64) -> String {
        let mut hasher = Sha1::new();
        hasher.update(format!("{}{}{}", app_secret, nonce, cur_time));
        format!("{:x}", hasher.finalize())
    }

    pub async fn checksum_request(&self,auth_header:&AuthHeader)->Result<(AgentInfo,bool),AppError>{
        let agent = self.find_by_app_key(&auth_header.app_key).await?;
        let signature = self.generate_checksum(&agent.app_secret, &auth_header.nonce, auth_header.timestamp);
        if signature != auth_header.signature {
            return Ok((agent,false));
        }
        return Ok((agent,true))
    }
    /// 初始化单例（仅运行一次）
    pub fn init(db: Database) {
        let instance = Self::new(db);
        AGENT_SERVICE
            .set(Arc::new(instance))
            .expect("AgentService already initialized");
    }

    /// 获取单例
    pub fn get() -> Arc<Self> {
        AGENT_SERVICE
            .get()
            .expect("AgentService is not initialized")
            .clone()
    }
}
static AGENT_SERVICE: OnceCell<Arc<AgentService>> = OnceCell::new();


#[derive(Debug, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct AuthHeader {
    pub app_key: String,
    pub nonce: String,
    pub timestamp: i64,
    pub signature: String,
}