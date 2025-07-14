use crate::manager::user_manager_core::{UserManager, UserManagerOpt};
use crate::protocol::msg::auth::DeviceType;
use anyhow::Result;
use common::UserId;
use common::config::AppConfig;
use common::repository_util::{BaseRepository, Repository};
use common::util::common_utils::build_md5_with_key;
use common::util::date_util::now;
use mongodb::Database;
use mongodb::bson::doc;
use once_cell::sync::OnceCell;
use std::sync::Arc;
use crate::protocol::common::ClientEntity;

#[derive(Debug)]
pub struct ClientService {
    pub dao: BaseRepository<ClientEntity>,
}

impl ClientService {
    pub fn new(db: Database) -> Self {
        let collection = db.collection("client");
        Self {
            dao: BaseRepository::new(db, collection.clone()),
        }
    }
    pub async fn new_data(
        &self,
        user_id: &UserId,
        name: String,
        avatar: String,
        username: String,
        password: String,
    ) -> Result<ClientEntity> {
        let mut user = ClientEntity::default();

        user.name = name;
        user.avatar = avatar;
        user.enable = true;
        user.uid = user_id.to_string();
        let md5_key = &AppConfig::get().get_sys().md5_key;
        if !password.is_empty() {
            user.password = build_md5_with_key(&password, &md5_key);
        }
        if !username.is_empty() {
            user.username = username;
        }
        self.dao.insert(&user).await?;
        Ok(user)
    }

    pub async fn find_by_user_id(&self, user_id: &UserId) -> Result<Option<ClientEntity>> {
        let option = UserManager::get().get_user_info(user_id).await?;
        Ok(option)
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
        INSTANCE
            .set(Arc::new(instance))
            .expect("INSTANCE already initialized");
    }

    /// 获取单例
    pub fn get() -> Arc<Self> {
        INSTANCE.get().expect("INSTANCE is not initialized").clone()
    }
}
static INSTANCE: OnceCell<Arc<ClientService>> = OnceCell::new();
