use crate::config::DatabaseConfig;
use anyhow::{Result, anyhow};
use mongodb::options::ClientOptions;
use mongodb::{Client, Database};
use once_cell::sync::OnceCell;

#[derive(Clone)]
pub struct Db {
    pub db: Database,
}

impl Db {
    /// 创建新实例
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// 初始化 MongoDB 数据库连接（全局只允许初始化一次）
    pub async fn init(config: &DatabaseConfig) -> Result<()> {
        let client_options = ClientOptions::parse(&config.url)
            .await
            .map_err(|e| anyhow!("MongoDB URI parse error: {}", e))?;

        let client = Client::with_options(client_options)
            .map_err(|e| anyhow!("MongoDB client init error: {}", e))?;

        let db = client.database(&config.db_name);
        let instance = Self::new(db);

        INSTANCE.set(instance).map_err(|_| anyhow!("MongoDB already initialized"))
    }

    /// 获取全局数据库实例引用
    ///
    /// # Panics
    /// 若未初始化则 panic
    pub fn get() -> &'static Database {
        &INSTANCE.get().expect("MongoDB is not initialized").db
    }
}

// 全局单例容器（私有）
static INSTANCE: OnceCell<Db> = OnceCell::new();
