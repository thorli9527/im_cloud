use crate::service::rpc::arb_service_rpc_client::ArbClient;
use crate::socket::socket_manager::SocketManager;
use biz_service::protocol::common::CommonResp;
use biz_service::protocol::rpc::arb_client::arb_client_service_server::{ArbClientService, ArbClientServiceServer};
use biz_service::protocol::rpc::arb_client::UpdateVersionReq;
use biz_service::protocol::rpc::arb_models::{ListAllNodesResponse, NodeInfo, NodeType, QueryNodeReq, SyncListGroup};
use common::config::AppConfig;
use futures::future::err;
use once_cell::sync::OnceCell;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::{async_trait, Request, Response, Status};

pub struct ArbClientRpcServiceImpl {
    pub socket_list: Arc<RwLock<Vec<NodeInfo>>>,
}
impl ArbClientRpcServiceImpl {
    pub async fn start() {
        ArbClient::init().await.expect("init arb_client error");
        // 读取配置文件
        let app_cfg = AppConfig::get();
        let addr = SocketAddr::from_str(&app_cfg.get_shard().client_addr.unwrap()).expect("Invalid address");
        let svc = ArbClientRpcServiceImpl {
            socket_list: Arc::new(RwLock::new(Vec::new())),
        };

        tonic::transport::Server::builder().add_service(ArbClientServiceServer::new(svc)).serve(addr).await.expect("Failed to start server");
        log::warn!("ArbSocketServiceServer started");
    }
}
#[async_trait]
impl ArbClientService for ArbClientRpcServiceImpl {
    async fn update_version(&self, request: Request<UpdateVersionReq>) -> Result<Response<CommonResp>, Status> {
        todo!()
    }

    async fn sync_data(&self, request: Request<SyncListGroup>) -> Result<Response<CommonResp>, Status> {
        todo!()
    }

    async fn flush_nodes(&self, request: Request<()>) -> Result<Response<CommonResp>, Status> {
        todo!()
    }

    // async fn flush_shard_list(&self, request: Request<()>) -> Result<Response<CommonResp>, Status> {
    //     let arb_client = ArbClient::get();
    //     let mut client = arb_client.write().await;
    //     let request = QueryNodeReq {
    //         node_type: NodeType::SocketNode as i32,
    //     };
    //     let result = client.init_shard_kafka_list().await;
    //     match result {
    //         Ok(_) => Ok(Response::new(CommonResp {
    //             success: true,
    //             message: "Shard client list refreshed".to_string(),
    //         })),
    //         Err(e) => Err(Status::internal("Failed to refresh socket list ")),
    //     }
    // }
    //
    // async fn flush_socket_list(&self, request: Request<()>) -> Result<Response<CommonResp>, Status> {
    //     let arb_client = ArbClient::get();
    //     let mut client = arb_client.write().await;
    //     let request = QueryNodeReq {
    //         node_type: NodeType::SocketNode as i32,
    //     };
    //     match client.arb_client.list_all_nodes(request).await {
    //         Ok(data) => {
    //             // let
    //             let nodes = data.into_inner().nodes;
    //             let mut list_guard = self.socket_list.write().await;
    //             list_guard.clear();
    //             list_guard.extend(nodes.clone());
    //             SocketManager::dispatch_mislocated_connections(nodes).await.unwrap();
    //             Ok(Response::new(CommonResp {
    //                 success: true,
    //                 message: "Shard client list refreshed".to_string(),
    //             }))
    //         }
    //         Err(err) => {
    //             log::error!("[ArbSocketRpcServiceImpl] Failed to refresh socket list: {}", err);
    //             Err(Status::internal(format!("Failed to refresh socket list: {}", err)))
    //         }
    //     }
    // }
}
