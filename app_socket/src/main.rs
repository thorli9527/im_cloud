use app_socket::manager;
use app_socket::manager::socket_manager::{get_socket_manager, SocketManager};
use app_socket::server::start_server;
use biz_service::biz_service::kafka_service::KafkaService;
use common::config::AppConfig;
use common::errors::AppError;
use deadpool_redis::{Pool, PoolConfig};
use futures::StreamExt;
use log::LevelFilter;
use mongodb::options::ClientOptions;
use mongodb::{Client, Database};
use std::str::FromStr;
use std::sync::Arc;
use tokio::net::TcpListener;

/// 写通道类型，用于发送 protobuf 编码好的消息

/// 全局连接池（连接 ID -> 写通道）

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    AppConfig::init(&"socket-config.toml".to_string());
    let config = AppConfig::get();
    //初始化日志
    init_log(&config);
    let bind_cfg = format!("{}:{}", &config.server.host, &config.server.port);
    let listener = TcpListener::bind(bind_cfg).await?;
    let pool = build_redis_pool(&config);
    let db = init_mongo_db(&config).await;
    KafkaService::init(&config.kafka).await;
    biz_service::init_service(db);
    biz_service::manager::init(pool,true);
    let manager: Arc<SocketManager> = get_socket_manager();
    tokio::spawn(manager::job_manager::start_heartbeat_cleaner(manager.clone(), 30)); // 30秒无心跳视为断线
    start_server(listener, config.kafka.clone()).await
}

pub fn build_redis_pool(config: &AppConfig) -> Pool {
    // 从应用配置中获取 Redis URL
    let mut cfg = deadpool_redis::Config::from_url(config.redis.url.clone());

    // 设置连接池的配置参数
    cfg.pool = Some(PoolConfig {
        max_size: 16,                   // 最大连接数
        timeouts: Default::default(),   // 使用默认的超时时间
        queue_mode: Default::default(), // 使用默认的队列模式
    });
    // 创建并返回连接池
    cfg.create_pool(Some(deadpool_redis::Runtime::Tokio1)).expect("Failed to create Redis connection pool")
}



pub fn init_log(config: &AppConfig) -> Result<(), AppError> {
    let mut builder = env_logger::Builder::new();
    let log_level = &config.sys.log_leve;
    let mut filter = builder.filter(None, LevelFilter::from_str(log_level).unwrap());
    filter.init();
    Ok(())
}

pub async fn init_mongo_db(config: &AppConfig) -> Database {
    let client_options = ClientOptions::parse(config.database.url.clone()).await.expect("MongoDB URI ERROR");
    // 创建 MongoDB 客户端
    let client = Client::with_options(client_options).expect("CLIENT MongoDB ERROR");
    // 获取数据库句柄（例如，名为 "mydb" 的数据库）
    client.database(&config.database.db_name)
}
