use crate::biz_const::redis_const::CLIENT_TOKEN_KEY;
use crate::entitys::client_entity::ClientInfo;
use common::errors::AppError;
use common::redis::redis_template::RedisTemplate;
use common::repository_util::{BaseRepository, Repository};
use common::util::common_utils::as_ref_to_string;
use common::util::date_util::now;
use mongodb::bson::doc;
use mongodb::bson::oid::ObjectId;
use mongodb::Database;
use once_cell::sync::OnceCell;
use std::sync::Arc;

#[derive(Debug)]
pub struct ClientService {
    pub dao: BaseRepository<ClientInfo>,
}

impl ClientService {
    pub fn new(db: Database) -> Self {
        let collection = db.collection("user_profile");
        Self { dao: BaseRepository::new(db, collection.clone()) }
    }
    pub async fn new_data(&self, agent_id: String, user_id: String, name: String, avatar: Option<String>) -> Result<ClientInfo, AppError> {
        let mut user = ClientInfo::default();
        user.agent_id = agent_id;
        user.name = name;
        user.avatar_url = avatar;
        user.enable = true;
        user.user_id = user_id;
        self.dao.insert(&user).await?;
        Ok(user)
    }

    pub async fn find_by_user_id(&self, user_id: impl AsRef<str>) -> Result<ClientInfo, AppError> {
        let result = self.dao.find_one(doc! {"user_id":as_ref_to_string(user_id)}).await?;
        match result {
            Some(mut client) => {
                match client.message_expired_at {  
                    Some(end_time)=>{
                        if end_time>now(){
                            client.message_status=true;
                        }
                    }
                    (_)=>{}
                }
                return Ok(client);
            }
            (_) => return Ok(ClientInfo::default()),
        }
    }
    
    pub async fn enable_message(&self, id: impl AsRef<str>)->Result<(), AppError>{
        let object_id = ObjectId::parse_str(as_ref_to_string(id)).unwrap();
        let filter = doc! {"_id":object_id};
        let update = doc! {
            "$unset": {
                "message_expired_at": ""
            },
             "$set": {
                "message_status": false
            }
        };
        self.dao.collection.update_one(filter, update).await?;
        Ok(())
    }

    pub async fn verify_token(&self, token: impl AsRef<str> + std::fmt::Display) -> Result<bool, AppError> {
        let redis_service = RedisTemplate::get();
        let key = format!("{} {}", CLIENT_TOKEN_KEY, token);
        let exists = redis_service.exists(key).await?;
        return Ok(exists);
    }
    pub async fn find_client_by_token(&self, token: impl AsRef<str> + std::fmt::Display) -> Result<ClientInfo, AppError> {
        let redis_service = RedisTemplate::get();
        let key = format!("{} {}", CLIENT_TOKEN_KEY, token);
        let result = redis_service.get_key_value::<ClientInfo>(key).await?;
        return Ok(result);
    }
    pub fn init(db: Database) {
        let instance = Self::new(db);
        INSTANCE.set(Arc::new(instance)).expect("INSTANCE already initialized");
    }

    /// 获取单例
    pub fn get() -> Arc<Self> {
        INSTANCE.get().expect("INSTANCE is not initialized").clone()
    }
}
static INSTANCE: OnceCell<Arc<ClientService>> = OnceCell::new();
