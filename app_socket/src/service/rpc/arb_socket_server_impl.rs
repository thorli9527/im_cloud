use crate::protocol::rpc_arb_socket::arb_socket_service_server::{ArbSocketService, ArbSocketServiceServer};
use crate::service::rpc::arb_service_rpc_client::ArbClient;
use biz_service::protocol::common::CommonResp;
use common::config::AppConfig;
use once_cell::sync::OnceCell;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::{async_trait, Request, Response, Status};

pub struct ArbSocketRpcServiceImpl{
}
impl ArbSocketRpcServiceImpl {
    pub fn new() -> Self {
        Self {}
    }
    pub async fn start(&self) {

        ArbClient::init().await.expect("init arb_client error");
        // 读取配置文件
        let app_cfg = AppConfig::get();
        let addr = SocketAddr::from_str(&app_cfg.get_shard().server_host.unwrap()).expect("Invalid address");
        let svc = ArbSocketRpcServiceImpl {};

        tonic::transport::Server::builder()
            .add_service(ArbSocketServiceServer::new(svc))
            .serve(addr)
            .await
            .expect("Failed to start server");
        log::warn!("ArbSocketServiceServer started");
    }
}
#[async_trait]
impl ArbSocketService for ArbSocketRpcServiceImpl {
    async fn flush_shard_list(&self, request: Request<()>) -> Result<Response<CommonResp>, Status> {
        log::info!("[ArbSocketRpcServiceImpl] Received flush_shard_list request.");
        let arb_client = ArbClient::get();
        let mut client = arb_client.write().await;

        match client.init_shard_clients().await {
            Ok(_) => {
                Ok(Response::new(CommonResp {
                    success: true,
                    message: "Shard client list refreshed".to_string(),
                }))
            }
            Err(err) => {
                log::error!("[ArbSocketRpcServiceImpl] Failed to refresh shard clients: {}", err);
                Err(Status::internal(format!("Failed to refresh shard list: {}", err)))
            }
        }
    }
}

