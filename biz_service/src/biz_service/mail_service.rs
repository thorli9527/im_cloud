use crate::entitys::mail_entity::MailEntity;
use async_trait::async_trait;
use common::repository_util::{BaseRepository, Repository};
use mongodb::Database;
use once_cell::sync::OnceCell;
use std::sync::Arc;

#[async_trait]
pub trait MailServiceTrait: Send + Sync {
    async fn send_mail(&self, mail: &MailEntity) -> anyhow::Result<()>;
}

#[derive(Debug)]
pub struct MailService {
    pub dao: BaseRepository<MailEntity>,
}

impl MailService {
    pub async fn new(db: Database) -> Self {
        let collection = db.collection("mail");
        Self {
            dao: BaseRepository::new(db, collection.clone(), "mail").await,
        }
    }

    pub async fn init(db: Database) {
        let instance = Self::new(db).await;
        INSTANCE.set(Arc::new(instance)).expect("INSTANCE already initialized");
    }

    /// 获取单例
    pub fn get() -> Arc<Self> {
        INSTANCE.get().expect("INSTANCE is not initialized").clone()
    }
}

static INSTANCE: OnceCell<Arc<MailService>> = OnceCell::new();

#[async_trait]
impl MailServiceTrait for MailService {
    async fn send_mail(&self, mail: &MailEntity) -> anyhow::Result<()> {
        self.dao.insert(mail).await?;
        Ok(())
    }
}
