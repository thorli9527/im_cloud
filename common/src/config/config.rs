// use crate::redis::redis_template::RedisTemplate;
use mongodb::Database;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub server: ServerConfig,
    pub sys: SysConfig,
    pub cache:CacheConfig
}
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
    // pub redis_template: RedisTemplate,
}
