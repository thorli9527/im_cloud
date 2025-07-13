use crate::manager::shard_job::ManagerJob;
use crate::manager::shard_manager::ShardManager;
use actix_web::middleware::Logger;
use actix_web::{App, HttpServer};
use common::config::AppConfig;
use common::errors::AppError;
use deadpool_redis::{Pool, PoolConfig};
use mongodb::options::ClientOptions;
use mongodb::{Client, Database};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::log::{warn, LevelFilter};

mod service;
mod manager;
mod protocol;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    AppConfig::init(&"group-config.toml".to_string());
    // 读取配置文件
    let app_cfg = AppConfig::get();
    //初始化日志
    init_log(&app_cfg.clone());
    let address_and_port = format!(
        "{}:{}",
        &app_cfg.get_server().host,
        &app_cfg.get_server().port
    );
    warn!("Starting server on {}", address_and_port);
    biz_service::init_service(init_mongo_db(&app_cfg).await);
    // 2. 构建 ShardManager 实例
    let config = app_cfg.clone().shard.clone().unwrap();
    let mut job = ManagerJob::new(config.clone());
    // 启动任务
    job.start().await;
    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            // 配置 控制器
            .configure(|cfg| {
                // handlers::configure(cfg);
            })
    })
    .keep_alive(actix_web::http::KeepAlive::Timeout(
        std::time::Duration::from_secs(600),
    )) // 允许 10 分钟超时
    .bind(address_and_port)?
    .run()
    .await
}

pub fn init_log(config: &AppConfig) -> Result<(), AppError> {
    let mut builder = env_logger::Builder::new();
    let log_level = &config.get_sys().log_leve;
    let mut filter = builder.filter(None, LevelFilter::from_str(log_level).unwrap());
    filter.init();
    Ok(())
}

pub fn build_redis_pool(config: &AppConfig) -> Pool {
    // 从应用配置中获取 Redis URL
    let mut cfg = deadpool_redis::Config::from_url(config.get_redis().url.clone());

    // 设置连接池的配置参数
    cfg.pool = Some(PoolConfig {
        max_size: 16,                   // 最大连接数
        timeouts: Default::default(),   // 使用默认的超时时间
        queue_mode: Default::default(), // 使用默认的队列模式
    });
    // 创建并返回连接池
    cfg.create_pool(Some(deadpool_redis::Runtime::Tokio1))
        .expect("Failed to create Redis connection pool")
}

pub async fn init_mongo_db(config: &AppConfig) -> Database {
    let client_options = ClientOptions::parse(config.get_database().url.clone())
        .await
        .expect("MongoDB URI ERROR");
    // 创建 MongoDB 客户端
    let client = Client::with_options(client_options).expect("CLIENT MongoDB ERROR");
    // 获取数据库句柄（例如，名为 "mydb" 的数据库）
    client.database(&config.get_database().db_name)
}
