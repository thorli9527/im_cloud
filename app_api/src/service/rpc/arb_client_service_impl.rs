use crate::service::rpc::arb_server_client::ArbServerClient;
use biz_service::protocol::common::CommonResp;
use biz_service::protocol::rpc::arb_models::{ListAllNodesResponse, NodeType, QueryNodeReq, RegRequest, SyncListGroup};
use common::config::AppConfig;
use common::util::common_utils::hash_index;
use common::util::date_util::now;
use log::info;

use biz_service::biz_service::kafka_socket_service::KafkaService;
use biz_service::protocol::rpc::arb_models::NodeType::SocketNode;
use biz_service::util::node_util::NodeUtil;
use once_cell::sync::OnceCell;
use std::net::SocketAddr;
use std::ops::Deref;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tonic::{transport, Request, Response, Status};
use biz_service::protocol::rpc::arb_client::arb_client_service_server::{ArbClientService, ArbClientServiceServer};
use biz_service::protocol::rpc::arb_client::UpdateVersionReq;

/// arb 组 客户端接口
#[derive(Debug, Clone)]
pub struct ArbClientServiceImpl {
    pub arb_server_client: Arc<ArbServerClient>,
}
impl ArbClientServiceImpl {
    pub async fn init() {
        NodeUtil::init().await;
        // 读取配置文件
        let app_cfg = AppConfig::get();
        let addr = SocketAddr::from_str(&app_cfg.get_shard().server_addr.unwrap()).expect("Invalid address");

        // 创建服务实例并放入全局单例
        let mut svc = Self {
            arb_server_client: Arc::new(ArbServerClient::new().await),
        };

        INSTANCE.set(Arc::new(svc.clone())).expect("INSTANCE already initialized");

        // 向服务端注册节点
        let request = RegRequest {
            node_type: NodeType::SocketGateway as i32,
            node_addr: addr.to_string(),
            kafka_addr: None,
        };

        svc.arb_server_client.client.clone().register_node(request).await.expect("reg socket gateway error");

        let response = svc
            .arb_server_client
            .client
            .clone()
            .list_all_nodes(QueryNodeReq {
                node_type: NodeType::SocketNode as i32,
            })
            .await
            .expect("list_all_nodes .error");
        let node_util = NodeUtil::get();
        node_util.await.push_list(SocketNode, response.into_inner().nodes);
        // 启动 gRPC 服务
        tonic::transport::Server::builder()
            .add_service(ArbClientServiceServer::new(svc.clone())) // ✅ 这里使用 Arc<Self>
            .serve(addr)
            .await
            .expect("Failed to start server");

        log::warn!("ArbGroupServiceServer started");
    }

    /// 获取单例
    pub fn get() -> Arc<Self> {
        INSTANCE.get().expect("INSTANCE is not initialized").clone()
    }
}

static INSTANCE: OnceCell<Arc<ArbClientServiceImpl>> = OnceCell::new();
#[tonic::async_trait]
impl ArbClientService for ArbClientServiceImpl {
    async fn update_version(&self, request: Request<UpdateVersionReq>) -> Result<Response<CommonResp>, Status> {
        Ok(Response::new(CommonResp {
            success: true,
            message: "".to_string(),
        }))
    }

    async fn sync_data(&self, request: Request<SyncListGroup>) -> Result<Response<CommonResp>, Status> {
        Ok(Response::new(CommonResp {
            success: true,
            message: "".to_string(),
        }))
    }

    async fn flush_nodes(&self, _: Request<()>) -> Result<Response<CommonResp>, Status> {
        let response = self
            .arb_server_client
            .client
            .clone()
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
