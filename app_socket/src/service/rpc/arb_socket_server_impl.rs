use std::sync::Arc;
use once_cell::sync::OnceCell;
use tokio::sync::RwLock;
use tonic::{async_trait, Request, Response, Status};
use biz_service::protocol::common::CommonResp;
use crate::service::rpc::arb_service_rpc_client::ArbClient;
use crate::protocol::rpc_arb_socket::arb_socket_service_server::ArbSocketService;

pub struct ArbSocketRpcServiceImpl{
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

