mod biz_service;
mod protocol;

use crate::biz_service::rpc::arb_service_impl::ArbiterServiceImpl;
use crate::protocol::rpc_arb_server::arb_server_rpc_service_server::ArbServerRpcServiceServer;
use common::config::AppConfig;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    AppConfig::init(&"arb-config.toml".to_string());
    // 读取配置文件
    let app_cfg = AppConfig::get();
    let addr = SocketAddr::from_str(&app_cfg.get_shard().server_host.unwrap())?;
    let svc = ArbiterServiceImpl {
        shard_nodes: Arc::new(Default::default()),
        socket_nodes: Arc::new(Default::default()),
    };

    tonic::transport::Server::builder()
        .add_service(ArbServerRpcServiceServer::new(svc))
        .serve(addr)
        .await?;

    Ok(())
}
