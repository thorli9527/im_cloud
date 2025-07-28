use app_socket::service::rpc::arb_socket_server_impl::ArbSocketRpcServiceImpl;
use app_socket::socket::socket_server::start_server;
use biz_service::biz_service::kafka_socket_service::KafkaService;
use common::config::AppConfig;
use log::warn;
use tokio::net::TcpListener;

/// 写通道类型，用于发送 protobuf 编码好的消息

/// 全局连接池（连接 ID -> 写通道）

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    AppConfig::init(&"./app_socket/socket-config.toml".to_string()).await;
    let config = AppConfig::get();
    //初始化 kafka
    KafkaService::init(&config.get_kafka()).await;
    //初始化业务
    biz_service::init_service().await;
    biz_service::manager::init();
    // let manager: Arc<SocketManager> = get_socket_manager();
    // tokio::spawn(job_manager::start_heartbeat_cleaner(manager.clone(), 30));

    // ✅ 并行启动 RPC 服务
    tokio::spawn(async {
        ArbSocketRpcServiceImpl::start().await;
    });
    //socket-web-server
    let bind_cfg = &config.socket.clone().unwrap().node_addr;
    println!(" - Socket 地址: {}", bind_cfg);
    warn!("socket-web-server bind: {}", bind_cfg.clone());
    let listener = TcpListener::bind(bind_cfg).await?;
    start_server(listener, &config.get_kafka().clone()).await
}
