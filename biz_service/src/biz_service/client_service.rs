use crate::entitys::client_entity::ClientInfo;
use crate::manager::common::UserId;
use crate::manager::user_manager_core::{UserManager, UserManagerOpt};
use anyhow::Result;
use common::repository_util::{BaseRepository, Repository};
use common::util::date_util::now;
use mongodb::Database;
use mongodb::bson::doc;
use once_cell::sync::OnceCell;
use std::sync::Arc;
use crate::protocol::auth::DeviceType;

#[derive(Debug)]
pub struct ClientService {
    pub dao: BaseRepository<ClientInfo>,
}

impl ClientService {
    pub fn new(db: Database) -> Self {
        let collection = db.collection("client");
        Self { dao: BaseRepository::new(db, collection.clone()) }
    }
    pub async fn new_data(&self, agent_id: String, user_id: &UserId, name: String, avatar: Option<String>) -> Result<ClientInfo> {
        let mut user = ClientInfo::default();
        user.agent_id = agent_id;
        user.name = name;
        user.avatar = avatar;
        user.enable = true;
        user.uid = user_id.to_string();
        user.agent_id_uid = format!("{}_{}", user.agent_id, user.uid);
        self.dao.insert(&user).await?;
        Ok(user)
    }

    pub async fn find_by_user_id(&self, agent_id: impl AsRef<str>, user_id: &UserId) -> Result<Option<ClientInfo>> {
        let option = UserManager::get().get_user_info(agent_id.as_ref(), user_id).await?;
        Ok(option.map(|mut client| {
            client.message_status = matches!(client.message_expired_at, Some(end) if end > now());
            client
        }))
    }

    pub async fn enable_message(&self, agent_id: &str, user_id: &UserId, status: bool) -> Result<()> {
        let key = format!("{}_:{}", agent_id, user_id);
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
        let option = user_manager.get_user_info(agent_id.as_ref(), user_id).await?;
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

    pub async fn build_token(&self, agent_id: &str, uid: &UserId, device_type: DeviceType) -> Result<()> {
        let user_manager = UserManager::get();
        user_manager.build_token(agent_id, uid, device_type).await?;
        Ok(())
    }
    pub async fn find_client_by_token(&self, token: &str) -> Result<Option<ClientInfo>> {
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
