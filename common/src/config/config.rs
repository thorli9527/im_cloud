use config::Config;
// use crate::redis::redis_template::RedisTemplate;
use mongodb::Database;
use once_cell::sync::OnceCell;
use serde::Deserialize;
use std::sync::Arc;
#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub server: ServerConfig,
    pub sys: SysConfig,
    pub cache:CacheConfig,
    pub kafka: KafkaConfig,
}


impl AppConfig{

    pub fn new(file:&String) -> Self {
        let config = Config::builder()
            .add_source(config::File::with_name(file).required(true))
            .add_source(config::Environment::with_prefix("APP").separator("_"))
            .build()
            .expect("Failed to build configuration");
        let cfg = config.try_deserialize::<AppConfig>().expect("Failed to deserialize configuration");
        return cfg;
    }
    pub fn init(file:&String) {
        let instance = Self::new(&file);
        INSTANCE.set(Arc::new(instance)).expect("INSTANCE already initialized");
    }

    /// 获取单例
    pub fn get() -> Arc<Self> {
        INSTANCE.get().expect("INSTANCE is not initialized").clone()
    }
    //强制下线
}
static INSTANCE: OnceCell<Arc<AppConfig>> = OnceCell::new();
#[derive(Debug, Deserialize, Clone)]
pub struct CacheConfig{
    pub node_id: usize,
    pub node_total:usize
}
#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub db_name: String,
}
#[derive(Debug, Deserialize, Clone)]
pub struct RedisConfig {
    pub url: String,
}
#[derive(Debug, Deserialize, Clone)]
pub struct SysConfig {
    //全局日志级别
    pub log_leve: String,
    //默认文件路径
    pub upload_path: String,
    //md5混淆 key
    pub md5_key: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}
#[derive(Debug, Clone)]
pub struct ServerRes {
    pub db: Database,
}
#[derive(Debug, Deserialize, Clone)]
pub struct KafkaConfig {
    pub brokers: String,
    pub topic_single: String,
    pub topic_group: String,
}


