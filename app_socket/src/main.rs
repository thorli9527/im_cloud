use app_socket::manager;
use app_socket::manager::socket_manager::{get_socket_manager, SocketManager};
use app_socket::socket::socket_server::start_server;
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
use app_socket::service::rpc::arb_service_rpc_client::ArbClient;

/// 写通道类型，用于发送 protobuf 编码好的消息

/// 全局连接池（连接 ID -> 写通道）

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    AppConfig::init(&"socket-config.toml".to_string()).await;
    let config = AppConfig::get();
    //初始化日志
    init_log(&config).expect("init log error");
   
    //初始化 kafka
    KafkaService::init(&config.get_kafka()).await;
    
    //初始化业务
    biz_service::manager::init();
    let manager: Arc<SocketManager> = get_socket_manager();
    tokio::spawn(manager::job_manager::start_heartbeat_cleaner(manager.clone(), 30));
  
    //初始化 arb-client
    let arb_client = ArbClient::init().await?;
    
    //socket-server
    let bind_cfg = format!("{}:{}", &config.get_server().host, &config.get_server().port);
    let listener = TcpListener::bind(bind_cfg).await?;
    start_server(listener, &config.get_kafka().clone()).await
}

pub fn init_log(config: &AppConfig) -> Result<(), AppError> {
    let mut builder = env_logger::Builder::new();
    let log_level = &config.get_sys().log_leve;
    let mut filter = builder.filter(None, LevelFilter::from_str(log_level).unwrap());
    filter.init();
    Ok(())
}
