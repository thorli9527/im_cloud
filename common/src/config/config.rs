use std::str::FromStr;
use crate::redis::redis_pool::RedisPoolTools;
use crate::repository::db::Db;
use config::Config;
use once_cell::sync::OnceCell;
use serde::Deserialize;
use std::sync::Arc;
use log::LevelFilter;
use crate::errors::AppError;

#[derive(Debug, Deserialize, Clone,Default)]
pub struct AppConfig {
    pub database: Option<DatabaseConfig>,
    pub redis: Option<RedisConfig>,
    pub server: Option<ServerConfig>,
    pub sys: Option<SysConfig>,
    pub cache: Option<CacheConfig>,
    pub kafka: Option<KafkaConfig>,
    pub shard: Option<ShardConfig>,
}
#[derive(Debug, Deserialize, Clone,Default)]
pub struct ShardConfig {
    pub shard_address: Option<String>,
    pub server_host: Option<String>,
}
impl AppConfig {
    pub fn new(file: &String) -> Self {
        let config = Config::builder()
            .add_source(config::File::with_name(file).required(true))
            .add_source(config::Environment::with_prefix("APP").separator("_"))
            .build()
            .expect("Failed to build configuration");
        let cfg = config.try_deserialize::<AppConfig>().expect("Failed to deserialize configuration");
        return cfg;
    }
    pub  async fn init(file: &String) {
        let instance = Self::new(&file);
        if instance.redis.is_some() {
            let redis_config=instance.clone().redis.unwrap();
            RedisPoolTools::init(&redis_config.url).expect("init redis error");
        }
        
        if instance.database.is_some() {
            let database_config=instance.clone().database.unwrap();
            Db::init(&database_config).await.expect("init database error");
        }
        
        if instance.sys.is_some(){
            let log_lovel=instance.clone().sys.unwrap().log_leve;
            if log_lovel.is_some(){
                init_log(&log_lovel.clone().unwrap()).expect("init log error");
            }
            
        }
        INSTANCE.set(Arc::new(instance)).expect("INSTANCE already initialized");
    }
    
    pub fn get_database(&self) -> DatabaseConfig {
        self.database.clone().unwrap_or_default()
    }
    pub fn get_redis(&self) -> RedisConfig {
        self.redis.clone().unwrap_or_default()
    }
    pub fn get_server(&self) -> ServerConfig {
        self.server.clone().unwrap_or_default()
    }
    pub fn get_sys(&self) -> SysConfig {
        self.sys.clone().unwrap_or_default()
    }
    pub fn get_cache(&self) -> CacheConfig {
        self.cache.clone().unwrap_or_default()
    }
    pub fn get_kafka(&self) -> KafkaConfig {
        self.kafka.clone().unwrap_or_default()
    }
    pub fn get_shard(&self) -> ShardConfig {
        self.shard.clone().unwrap_or_default()
    }
    /// 获取单例
    pub fn get() -> Arc<Self> {
        INSTANCE.get().expect("INSTANCE is not initialized").clone()
    }
    //强制下线
}

pub fn init_log(log_lovel: &str) -> Result<(), AppError> {
    let mut builder = env_logger::Builder::new();
    let filter = builder.filter(None, LevelFilter::from_str(log_lovel).unwrap());
    filter.init();
    Ok(())
}
static INSTANCE: OnceCell<Arc<AppConfig>> = OnceCell::new();
#[derive(Debug, Deserialize, Clone,Default)]
pub struct CacheConfig {
    pub node_id: usize,
    pub node_total: usize,
}
#[derive(Debug, Deserialize, Clone,Default)]
pub struct DatabaseConfig {
    pub url: String,
    pub db_name: String,
}
#[derive(Debug, Deserialize, Clone,Default)]
pub struct RedisConfig {
    pub url: String,
}
#[derive(Debug, Deserialize, Clone,Default)]
pub struct SysConfig {
    //全局日志级别
    pub log_leve: Option<String>,
    //默认文件路径
    pub upload_path: Option<String>,
    //md5混淆 key
    pub md5_key: Option<String>,
}

#[derive(Debug, Deserialize, Clone,Default)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Clone,Default)]
pub struct KafkaConfig {
    pub brokers: String,
    pub topic_single: String,
    pub topic_group: String,
}
