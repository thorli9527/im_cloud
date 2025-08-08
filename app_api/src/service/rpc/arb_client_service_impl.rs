use biz_core::protocol::common::CommonResp;
use common::config::AppConfig;
use common::util::common_utils::hash_index;
use common::util::date_util::now;
use log::info;

use biz_core::service::rpc_server_client_service::ArbServerRpcServiceClientService;
use once_cell::sync::OnceCell;
use std::str::FromStr;
use std::sync::Arc;
use tonic::{transport, Request, Response, Status};
use biz_core::kafka_util::node_util::NodeUtil;
use biz_core::protocol::arb::arb_client::arb_client_service_server::{ArbClientService, ArbClientServiceServer};
use biz_core::protocol::arb::arb_client::UpdateVersionReq;
use biz_core::protocol::arb::arb_models::{NodeType, QueryNodeReq, RegRequest, SyncListGroup};
use biz_core::protocol::arb::arb_models::NodeType::SocketNode;

/// arb 组 客户端接口
#[derive(Debug, Clone)]
pub struct ArbClientServiceImpl {}
impl ArbClientServiceImpl {
    pub async fn start() {
        // 读取配置文件
        let rpc_server_service = ArbServerRpcServiceClientService::get();
        let mut client = rpc_server_service.client.lock().await;

        let client_addr = AppConfig::get().get_shard().clone().client_addr.unwrap();
        let client_services = Self {};
        // 向服务端注册节点
        let request = RegRequest {
            node_type: NodeType::SocketGateway as i32,
            node_addr: client_addr.clone(),
            kafka_addr: None,
        };

        client.register_node(request).await.expect("reg socket gateway error");

        let response = client
            .list_all_nodes(QueryNodeReq {
                node_type: NodeType::SocketNode as i32,
            })
            .await
            .expect("list_all_nodes.error");
        let node_util = NodeUtil::get();

        node_util.await.push_list(SocketNode, response.into_inner().nodes);

        tokio::spawn(async move {
            // 启动 gRPC 服务
            let addr = std::net::SocketAddr::from_str(&client_addr.clone())
                .expect("Invalid socket address");
            tonic::transport::Server::builder()
                .add_service(ArbClientServiceServer::new(client_services)) // ✅ 这里使用 Arc<Self>
                .serve(addr)
                .await
                .expect("Failed to start server");
        });

        log::warn!("ArbGroupServiceServer started");
    }

    /// 获取单例
    pub fn get() -> Arc<Self> {
        INSTANCE.get().expect("INSTANCE is not initialized").clone()
    }

    pub async fn init() -> anyhow::Result<()> {
        NodeUtil::init().await;
        ArbServerRpcServiceClientService::init().await?;
        ArbClientServiceImpl::start().await;
        Ok(())
    }
}

static INSTANCE: OnceCell<Arc<ArbClientServiceImpl>> = OnceCell::new();
#[tonic::async_trait]
impl ArbClientService for ArbClientServiceImpl {
    async fn update_version(
        &self,
        request: Request<UpdateVersionReq>,
    ) -> Result<Response<CommonResp>, Status> {
        Ok(Response::new(CommonResp {
            success: true,
            message: "".to_string(),
        }))
    }

    async fn sync_data(
        &self,
        request: Request<SyncListGroup>,
    ) -> Result<Response<CommonResp>, Status> {
        Ok(Response::new(CommonResp {
            success: true,
            message: "".to_string(),
        }))
    }

    async fn flush_nodes(&self, _: Request<()>) -> Result<Response<CommonResp>, Status> {
        let rpc_server_service = ArbServerRpcServiceClientService::get();
        let mut client = rpc_server_service.client.lock().await;
        let response = client
            .list_all_nodes(QueryNodeReq {
                node_type: NodeType::SocketNode as i32,
            })
            .await?;
        let node_util = NodeUtil::get();
        node_util.await.push_list(SocketNode, response.into_inner().nodes);
        Ok(Response::new(CommonResp {
            success: true,
            message: String::new(),
        }))
    }
}
