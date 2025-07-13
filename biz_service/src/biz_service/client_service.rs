use crate::entitys::client_entity::ClientEntity;
use crate::manager::user_manager_core::{UserManager, UserManagerOpt};
use anyhow::Result;
use common::repository_util::{BaseRepository, Repository};
use common::util::date_util::now;
use mongodb::bson::doc;
use mongodb::Database;
use once_cell::sync::OnceCell;
use std::sync::Arc;
use utoipa::openapi::security::Password;
use common::config::AppConfig;
use common::UserId;
use common::util::common_utils::{build_md5, build_md5_with_key};
use crate::protocol::msg::auth::DeviceType;

#[derive(Debug)]
pub struct ClientService {
    pub dao: BaseRepository<ClientEntity>,
}

impl ClientService {
    pub fn new(db: Database) -> Self {
        let collection = db.collection("client");
        Self { dao: BaseRepository::new(db, collection.clone()) }
    }
    pub async fn new_data(&self,  user_id: &UserId, name: String, avatar: Option<String>,username: Option<String>,password: Option<String>) -> Result<ClientEntity> {
        let mut user = ClientEntity::default();
        
        user.name = name;
        user.avatar = avatar;
        user.enable = true;
        user.uid = user_id.to_string();
        if password.is_some() {
            let md5_key=&AppConfig::get().get_sys().md5_key;
            let password=password.unwrap();
            if !password.is_empty(){
                user.password= Some(build_md5_with_key(&password,&md5_key));
            }
        }
        if username.is_some() {
            let username=username.unwrap();
            if !username.is_empty(){
                user.username= Some(username);
            }
        }
        self.dao.insert(&user).await?;
        Ok(user)
    }
    
    pub async fn find_by_user_id(&self,  user_id: &UserId) -> Result<Option<ClientEntity>> {
        let option = UserManager::get().get_user_info( user_id).await?;
        Ok(option.map(|mut client| {
            client.message_status = matches!(client.message_expired_at, Some(end) if end > now());
            client
        }))
    }

    pub async fn enable_message(&self,user_id: &UserId, status: bool) -> Result<()> {
        let key = format!("{}",  user_id);
        let filter = doc! {"agent_id_user_id":key};
        let update = doc! {
            "$unset": {
                "message_expired_at": ""
            },
             "$set": {
                "message_status": status
            }
        };
        self.dao.collection.update_one(filter, update).await?;
        let user_manager = UserManager::get();
        let option = user_manager.get_user_info( user_id).await?;
        if let Some(mut client) = option {
            client.message_expired_at = None;
            client.message_status = status;
            user_manager.sync_user(client).await?
        }
        Ok(())
    }

  

    pub async fn verify_token(&self, token: &str) -> Result<bool> {
        Ok(UserManager::get().verify_token(token).await?)
    }

    pub async fn lock(&self, agent_id: &str, user_id: &UserId) {
        //
    }

    pub async fn build_token(&self, uid: &UserId, device_type: &DeviceType) -> Result<()> {
        let user_manager = UserManager::get();
        user_manager.build_token(uid, device_type).await?;
        Ok(())
    }
    pub async fn find_client_by_token(&self, token: &str) -> Result<Option<ClientEntity>> {
        let user_manager = UserManager::get();
        Ok(user_manager.find_user_by_token(token).await?)
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
