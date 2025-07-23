mod protocol;
mod service;

use crate::protocol::rpc_arb_server::arb_server_rpc_service_server::ArbServerRpcServiceServer;
use crate::service::rpc::arb_service_impl::ArbiterServiceImpl;
use common::config::AppConfig;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use tracing::log;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    AppConfig::init(&"./app_arb/arb-config.toml".to_string()).await;
    // 读取配置文件
    let app_cfg = AppConfig::get();
    let addr = SocketAddr::from_str(&app_cfg.get_shard().server_host.unwrap())?;
    let svc = ArbiterServiceImpl { shard_nodes: Arc::new(Default::default()), socket_nodes: Arc::new(Default::default()), arb_version: Arc::new(AtomicU64::new(0)) };

    tonic::transport::Server::builder().add_service(ArbServerRpcServiceServer::new(svc)).serve(addr).await?;
    log::warn!("ArbServerRpcServiceServer started");
    Ok(())
}
