mod biz_service;
mod protocol;

use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;
use common::config::AppConfig;
use crate::biz_service::grpc::arb_service_impl::{ArbiterServiceImpl};
use crate::protocol::arbitration::arbiter_service_server::ArbiterServiceServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    AppConfig::init(&"arb-config.toml".to_string());
    // 读取配置文件
    let app_cfg = AppConfig::get();
    let addr = SocketAddr::from_str(&app_cfg.get_shard().server_host)?;
    let svc = ArbiterServiceImpl {
        shard_nodes: Arc::new(Default::default()),
    };

    tonic::transport::Server::builder()
        .add_service(ArbiterServiceServer::new(svc))
        .serve(addr)
        .await?;

    Ok(())
}
