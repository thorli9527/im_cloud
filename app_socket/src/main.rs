use app_socket::scheduler;
use app_socket::service::rpc::arb_client_service_impl::ArbClientServiceImpl;
use app_socket::socket::socket_server::start_server;
use common::config::AppConfig;
use log::warn;
use tokio::net::TcpListener;
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    AppConfig::init(&"./app_socket/socket-config.toml".to_string()).await;
    let config = AppConfig::get();
    //初始化 kafka
    // KafkaService::init(&config.get_kafka()).await;
    //初始化业务
    biz_service::init_service().await;
    //启动任务
    scheduler::configure();
    biz_service::manager::init();
    //arb server 与消费
    ArbClientServiceImpl::start().await;
    //socket-web-server
    let bind_cfg = &config.socket.clone().unwrap().node_addr;
    warn!("socket-web-server bind: {}", bind_cfg.clone());
    let listener = TcpListener::bind(bind_cfg).await?;
    start_server(listener, &config.get_kafka().clone()).await
}
