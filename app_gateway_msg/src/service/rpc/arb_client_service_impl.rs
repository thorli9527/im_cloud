use crate::service::rpc::arb_server_client::ArbServerClient;
use biz_service::protocol::common::CommonResp;
use biz_service::protocol::rpc::arb_client::arb_client_service_server::{ArbClientService, ArbClientServiceServer};
use biz_service::protocol::rpc::arb_client::UpdateVersionReq;
use biz_service::protocol::rpc::arb_models::{NodeType, QueryNodeReq, RegRequest, SyncListGroup};
use biz_service::util::node_util::NodeUtil;
use common::config::AppConfig;
use log::info;
use once_cell::sync::OnceCell;
use std::net::SocketAddr;
use std::ops::Deref;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::{transport::Server, Request, Response, Status};

/// ArbClientServiceImpl: 与 ArbServer 交互并提供 gRPC 服务接口
#[derive(Debug, Clone)]
pub struct ArbClientServiceImpl {
    /// ArbServer gRPC 客户端
    arb_server_client: Arc<ArbServerClient>,
    /// 本地节点列表缓存
    socket_node_list: Arc<RwLock<Vec<String>>>,
    group_node_list: Arc<RwLock<Vec<String>>>,
}

static INSTANCE: OnceCell<Arc<ArbClientServiceImpl>> = OnceCell::new();

impl ArbClientServiceImpl {
    /// 初始化 NodeUtil，注册自身并启动 gRPC 服务
    /// 初始化 NodeUtil，注册自身并启动 gRPC 服务
    pub async fn init() {
        // 初始化节点工具
        NodeUtil::init().await;

        // 读取配置并解析地址
        let app_cfg = AppConfig::get();
        let addr = SocketAddr::from_str(&app_cfg.get_shard().server_addr.unwrap()).expect("Invalid address");

        // 构造服务实例
        let svc = ArbClientServiceImpl {
            arb_server_client: Arc::new(ArbServerClient::new().await),
            socket_node_list: Arc::new(RwLock::new(Vec::new())),
            group_node_list: Arc::new(RwLock::new(Vec::new())),
        };

        // 存入单例
        let singleton = Arc::new(svc.clone());
        INSTANCE.set(singleton.clone()).expect("Instance already initialized");

        // 注册到 ArbServer
        let reg_req = RegRequest {
            node_type: NodeType::MsgGateway as i32,
            node_addr: addr.to_string(),
            kafka_addr: None,
        };
        singleton.arb_server_client.client.clone().register_node(reg_req).await.expect("Failed to register node");

        // 首次刷新 SocketNode 列表
        let socket_resp = singleton
            .arb_server_client
            .client
            .clone()
            .list_all_nodes(QueryNodeReq {
                node_type: NodeType::SocketNode as i32,
            })
            .await
            .expect("Failed to list socket nodes");
        NodeUtil::get().await.push_list(NodeType::SocketNode, socket_resp.into_inner().nodes);

        // 启动 gRPC 服务
        info!("Starting ArbClientServiceServer at {}", addr);
        Server::builder()
            // 传入实现了 ArbClientService 的普通实例，不是 Arc
            .add_service(ArbClientServiceServer::new((*singleton).clone()))
            .serve(addr)
            .await
            .expect("Failed to start gRPC server")
    }

    /// 获取单例
    pub fn get() -> Arc<Self> {
        INSTANCE.get().expect("Not initialized").clone()
    }
}
#[tonic::async_trait]
impl ArbClientService for ArbClientServiceImpl {
    async fn update_version(&self, _req: Request<UpdateVersionReq>) -> Result<Response<CommonResp>, Status> {
        Ok(Response::new(CommonResp {
            success: true,
            message: String::new(),
        }))
    }

    async fn sync_data(&self, _req: Request<SyncListGroup>) -> Result<Response<CommonResp>, Status> {
        Ok(Response::new(CommonResp {
            success: true,
            message: String::new(),
        }))
    }

    async fn flush_nodes(&self, _req: Request<()>) -> Result<Response<CommonResp>, Status> {
        // 拉取并更新 SocketNode 列表
        let socket_resp = self
            .arb_server_client
            .client
            .clone()
            .list_all_nodes(QueryNodeReq {
                node_type: NodeType::SocketNode as i32,
            })
            .await?;
        NodeUtil::get().await.push_list(NodeType::SocketNode, socket_resp.into_inner().nodes);

        // 拉取并更新 GroupNode 列表
        let group_resp = self
            .arb_server_client
            .client
            .clone()
            .list_all_nodes(QueryNodeReq {
                node_type: NodeType::GroupNode as i32,
            })
            .await?;
        NodeUtil::get().await.push_list(NodeType::GroupNode, group_resp.into_inner().nodes);

        Ok(Response::new(CommonResp {
            success: true,
            message: String::new(),
        }))
    }
}
