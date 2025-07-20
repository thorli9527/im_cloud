use app_socket::manager;
use app_socket::manager::socket_manager::{get_socket_manager, SocketManager};
use app_socket::service::rpc::arb_socket_server_impl::ArbSocketRpcServiceImpl;
use app_socket::socket::socket_server::start_server;
use biz_service::biz_service::kafka_socket_service::KafkaService;
use common::config::AppConfig;
use std::sync::Arc;
use tokio::net::TcpListener;

/// 写通道类型，用于发送 protobuf 编码好的消息

/// 全局连接池（连接 ID -> 写通道）

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    AppConfig::init(&"socket-config.toml".to_string()).await;
    let config = AppConfig::get();
    //初始化 kafka
    KafkaService::init(&config.get_kafka()).await;

    //初始化业务
    biz_service::manager::init();
    let manager: Arc<SocketManager> = get_socket_manager();
    tokio::spawn(manager::job_manager::start_heartbeat_cleaner(manager.clone(), 30));

    //启动 group rpc 服务
    ArbSocketRpcServiceImpl::start().await;

    //socket-web-server
    let bind_cfg = format!("{}:{}", &config.get_server().host, &config.get_server().port);
    let listener = TcpListener::bind(bind_cfg).await?;
    start_server(listener, &config.get_kafka().clone()).await
}
