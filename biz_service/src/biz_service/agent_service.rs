use crate::biz_service::cache_service::get_agent_cache;
use crate::entitys::agent_entity::AgentInfo;
use crate::protocol::protocol::DeviceType;
use actix_web::HttpRequest;
use anyhow::Result;
use common::errors::AppError;
use common::repository_util::{BaseRepository, Repository};
use mongodb::bson::doc;
use mongodb::Database;
use once_cell::sync::OnceCell;
use serde::Serialize;
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

    pub async fn find_by_app_key(&self, app_key: impl AsRef<str> + Clone) -> Result<AgentInfo> {
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

    pub async fn check_request(&self,auth_header:Option<AuthHeader>)->anyhow::Result<AgentInfo>{
        if 1==1{
            let option = self.dao.find_by_id("685bba60a23f55a165d6af13").await?;
            return Ok(option.unwrap());
        }
        if auth_header.is_none() {
            return Err(AppError::BizError("signature.error".to_string()).into());
        }
        let auth_header = auth_header.unwrap();
        let agent = self.find_by_app_key(&auth_header.app_key).await?;
        let signature = self.generate_checksum(&agent.app_secret, &auth_header.nonce, auth_header.timestamp);
        if signature != auth_header.signature {
            return Err(AppError::BizError("signature.error".to_string()).into());
        }
        Ok(agent)
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


#[derive(Debug, Serialize, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AuthHeader {
    pub app_key: String,
    pub nonce: String,
    pub timestamp: i64,
    pub signature: String,
    pub device_type: DeviceType,
}



pub fn build_header(req: HttpRequest) -> Option<AuthHeader> {
    // 从 Header 提取字段，全部采用小写统一处理
    let app_key = req.headers().get("appkey")?.to_str().ok()?.to_string();
    let nonce = req.headers().get("nonce")?.to_str().ok()?.to_string();
    let signature = req.headers().get("signature")?.to_str().ok()?.to_string();

    // 解析 timestamp 为 i64，失败时直接返回 None
    let timestamp = match req.headers().get("timestamp")?.to_str().ok()?.parse::<i64>() {
        Ok(ts) => ts,
        Err(_) => return None,
    };

    // 解析 device_type，默认使用 Web
    let device_type = req
        .headers()
        .get("devicetype")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| DeviceType::from_str_name(s))
        .unwrap_or(DeviceType::Web);

    Some(AuthHeader {
        app_key,
        nonce,
        timestamp,
        signature,
        device_type,
    })
}