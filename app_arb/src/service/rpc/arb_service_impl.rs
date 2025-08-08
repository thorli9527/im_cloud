use anyhow::__private::not;
use common::errors::AppError;
use common::util::date_util::now;
use dashmap::DashMap;
use std::clone;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::io::Empty;
use tonic::transport::Channel;
use tonic::{Code, IntoRequest, Request, Response, Status};
use tracing::log;
use biz_service::protocol::arb::arb_client::arb_client_service_client::ArbClientServiceClient;
use biz_service::protocol::arb::arb_client::UpdateVersionReq;
use biz_service::protocol::arb::arb_models::{BaseRequest, ListAllNodesResponse, NodeInfo, NodeType, QueryNodeReq, RegRequest, ShardState, UpdateShardStateRequest};
use biz_service::protocol::arb::arb_server::arb_server_rpc_service_server::ArbServerRpcService;
use biz_service::protocol::common::CommonResp;

/// 分片节点状态信息
/// 表示某个 vnode 当前的版本号、状态、归属节点及上次更新时间

/// ArbiterService 实现体，持有共享状态引用
#[derive(Clone, Default)]
pub struct ArbiterServiceImpl {
    pub shard_nodes: Arc<DashMap<String, NodeInfo>>,
    pub global_shard_state: Arc<tokio::sync::RwLock<ShardState>>,
    pub socket_nodes: Arc<DashMap<String, NodeInfo>>,
    pub socket_gateway_nodes: Arc<DashMap<String, NodeInfo>>,
    pub msg_gateway_nodes: Arc<DashMap<String, NodeInfo>>,
    pub msg_group_nodes: Arc<DashMap<String, NodeInfo>>,
    pub msg_friend_nodes: Arc<DashMap<String, NodeInfo>>,
}

impl ArbiterServiceImpl {
    pub fn new() -> Self {
        Self {
            shard_nodes: Arc::new(DashMap::new()),
            global_shard_state: Arc::new(tokio::sync::RwLock::new(ShardState::Normal)),
            socket_nodes: Arc::new(DashMap::new()),
            socket_gateway_nodes: Arc::new(DashMap::new()),
            msg_gateway_nodes: Arc::new(DashMap::new()),
            msg_group_nodes: Arc::new(DashMap::new()),
            msg_friend_nodes: Arc::new(DashMap::new()),
        }
    }
    pub async fn init_socket_clients(&self) -> Result<Vec<ArbClientServiceClient<Channel>>, Box<dyn std::error::Error + Send + Sync>> {
        let mut endpoints = Vec::new();
        for ref node in self.socket_nodes.iter() {
            let node_info = node.value();
            endpoints.push(node_info.node_addr.clone());
        }
        let mut clients = Vec::new();
        for endpoint in endpoints {
            let channel = Channel::from_shared(format!("http://{}", endpoint.clone()))?.connect().await?;
            let client = ArbClientServiceClient::new(channel);
            clients.push(client);
        }
        Ok(clients)
    }
    pub async fn init_clients(&self, node_type: NodeType) -> Result<Vec<ArbClientServiceClient<Channel>>, Box<dyn std::error::Error + Send + Sync>> {
        let mut endpoints = Vec::new();
        let arc = match node_type {
            NodeType::GroupNode => self.shard_nodes.clone(),
            NodeType::SocketNode => self.socket_nodes.clone(),
            NodeType::SocketGateway => self.socket_gateway_nodes.clone(),
            NodeType::MsgGateway => self.msg_gateway_nodes.clone(),
            NodeType::MesGroup => self.msg_group_nodes.clone(),
            NodeType::MsgFriend => self.msg_friend_nodes.clone(),
            _ => {
                return Err(AppError::BizError("Invalid node type".to_string()).into());
            }
        };

        for ref node in arc.iter() {
            let node_info = node.value();
            endpoints.push(node_info.node_addr.clone());
        }
        let mut clients = Vec::new();
        for endpoint in endpoints {
            let channel = Channel::from_shared(format!("http://{}", endpoint.clone()))?.connect().await?;
            let client = ArbClientServiceClient::new(channel);
            clients.push(client);
        }
        Ok(clients)
    }
}

#[tonic::async_trait]
impl ArbServerRpcService for ArbiterServiceImpl {
    // === 分片管理 ===

    async fn update_shard_state(&self, request: Request<UpdateShardStateRequest>) -> Result<Response<CommonResp>, Status> {
        let req = request.into_inner();
        let current_node_addr = req.node_addr.clone();
        let new_state = req.new_state;

        // 更新目标节点状态
        match self.shard_nodes.get_mut(&current_node_addr) {
            Some(mut entry) => {
                entry.state = new_state;
                entry.version += 1;
                entry.last_update_time = now() as u64;
                if entry.node_type == NodeType::GroupNode as i32 {
                    entry.total = self.shard_nodes.len() as i32;

                    // 如果新状态是 Normal，检查所有 shard 状态
                    if req.new_state == ShardState::Normal as i32 {
                        // 统计是否所有 shard 节点状态为 Normal
                        let all_normal = self.shard_nodes.iter().all(|entry| entry.value().state == ShardState::Normal as i32);

                        if all_normal {
                            // 升级 arb_version
                            let mut global_shard_state = self.global_shard_state.write().await;
                            *global_shard_state = ShardState::Normal;

                            //通知消息网关节点有变
                            let mut client_list = self.init_clients(NodeType::MsgGateway).await.expect("init clients error");
                            for client in client_list.iter_mut() {
                                client.flush_nodes(()).await?;
                            }
                            //通知socket网关节点有变
                            let mut client_list = self.init_clients(NodeType::SocketGateway).await.expect("init clients error");
                            for client in client_list.iter_mut() {
                                client.flush_nodes(()).await?;
                            }
                            //通知msg_friend节点有变
                            let mut client_list = self.init_clients(NodeType::MsgFriend).await.expect("init clients error");
                            for client in client_list.iter_mut() {
                                client.flush_nodes(()).await?;
                            }
                            //通知msg_group节点有变
                            let mut client_list = self.init_clients(NodeType::MesGroup).await.expect("init clients error");
                            for client in client_list.iter_mut() {
                                client.flush_nodes(()).await?;
                            }
                        } else {
                            log::info!("[ArbVersion] Node {} is Normal, but not all are Normal yet.", current_node_addr);
                        }
                    }
                }

                Ok(Response::new(CommonResp {
                    success: true,
                    message: format!("Updated node {} to state {:?}, version = {}", current_node_addr, entry.state, entry.version),
                }))
            }

            None => Err(Status::not_found(format!("Node {} not found.", current_node_addr))),
        }
    }

    /// 注册节点：如果 node_addr 已存在，则返回失败；否则分配唯一 index 并插入
    async fn register_node(&self, request: Request<RegRequest>) -> Result<Response<NodeInfo>, Status> {
        let req = request.into_inner();
        let node_addr = req.node_addr;
        // 当前时间戳（毫秒）
        let now = now() as u64;
        if req.node_type == NodeType::GroupNode as i32 {
            // 如果已存在，返回已有信息
            if let Some(node_info) = self.shard_nodes.get(&node_addr) {
                return Ok(Response::new(NodeInfo {
                    node_addr: node_info.node_addr.clone(),
                    total: self.shard_nodes.len() as i32,
                    version: node_info.version,
                    state: node_info.state,
                    node_type: req.node_type,
                    last_update_time: node_info.last_update_time,
                    kafka_addr: req.kafka_addr,
                }));
            }
            let mut global_shard_state = self.global_shard_state.write().await;
            *global_shard_state = ShardState::Ready;
            // 当前时间戳（毫秒）
            // 构建新 entry
            let mut entry = NodeInfo {
                node_addr: node_addr.clone(),
                total: self.shard_nodes.len() as i32 + 1,
                version: 0,
                node_type: req.node_type,
                state: ShardState::Preparing as i32,
                last_update_time: now,
                kafka_addr: req.kafka_addr,
            };

            for (mut item) in self.shard_nodes.iter_mut() {
                item.state = ShardState::Preparing as i32;
                let mut client_list = self.init_clients(NodeType::GroupNode).await.expect("init shard clients error");
                for mut client in client_list.iter_mut() {
                    let request = Request::new(UpdateVersionReq {
                        node_addr: item.node_addr.clone(),
                        version: item.version,
                        state: item.state,
                        last_update_time: now,
                        total: entry.total,
                    });
                    client.update_version(request).await.expect("update version error");
                }
            }

            // 插入
            self.shard_nodes.insert(node_addr.clone(), entry.clone());
            entry.total = self.shard_nodes.len() as i32;
            //打印信息
            // log::warn!("新增分片节点: {:?}", &entry);
            return Ok(Response::new(entry));
        }

        if req.node_type == NodeType::SocketNode as i32 {
            // 如果已存在，返回已有信息
            if let Some(node_info) = self.socket_nodes.get(&node_addr) {
                return Ok(Response::new(NodeInfo {
                    node_addr: node_info.node_addr.clone(),
                    total: self.socket_nodes.len() as i32,
                    version: node_info.version,
                    state: node_info.state,
                    node_type: req.node_type,
                    last_update_time: node_info.last_update_time,
                    kafka_addr: req.kafka_addr,
                }));
            }

            // 当前时间戳（毫秒）
            // 构建新 entry
            let mut entry = NodeInfo {
                node_addr: node_addr.clone(),
                total: self.socket_nodes.len() as i32 + 1,
                version: 0,
                node_type: req.node_type,
                state: ShardState::Normal as i32,
                last_update_time: now,
                kafka_addr: req.kafka_addr,
            };

            // 插入
            self.socket_nodes.insert(node_addr.clone(), entry.clone());
            entry.total = self.socket_nodes.len() as i32;
            let mut client_list = self.init_clients(NodeType::MsgGateway).await.expect("init clients error");
            for client in client_list.iter_mut() {
                client.flush_nodes(()).await?;
            }
            let mut client_list = self.init_clients(NodeType::SocketGateway).await.expect("init clients error");
            for client in client_list.iter_mut() {
                client.flush_nodes(()).await?;
            }

            let mut client_list = self.init_clients(NodeType::GroupNode).await.expect("init clients error");
            for client in client_list.iter_mut() {
                client.flush_nodes(()).await?;
            }
            let mut client_list = self.init_clients(NodeType::MsgFriend).await.expect("init clients error");
            for client in client_list.iter_mut() {
                client.flush_nodes(()).await?;
            }
            let mut client_list = self.init_clients(NodeType::MesGroup).await.expect("init clients error");
            for client in client_list.iter_mut() {
                client.flush_nodes(()).await?;
            }
            //打印信息
            // log::warn!("新增分片节点: {:?}", &entry);
            return Ok(Response::new(entry));
        }
        if req.node_type == NodeType::MsgGateway as i32 {
            // 构建新 entry
            let mut entry = NodeInfo {
                node_addr: node_addr.clone(),
                total: self.msg_gateway_nodes.len() as i32 + 1,
                version: 0,
                node_type: req.node_type,
                state: ShardState::Normal as i32,
                last_update_time: now,
                kafka_addr: req.kafka_addr,
            };
            // 插入
            self.msg_gateway_nodes.insert(node_addr.clone(), entry.clone());
            //打印信息
            // log::warn!("新增分片节点: {:?}", &entry);
            return Ok(Response::new(entry));
        }
        if req.node_type == NodeType::SocketGateway as i32 {
            // 构建新 entry
            let mut entry = NodeInfo {
                node_addr: node_addr.clone(),
                total: self.socket_gateway_nodes.len() as i32 + 1,
                version: 0,
                node_type: req.node_type,
                state: ShardState::Normal as i32,
                last_update_time: now,
                kafka_addr: req.kafka_addr,
            };
            self.socket_gateway_nodes.insert(node_addr.clone(), entry.clone());
            let mut client_list = self.init_clients(NodeType::GroupNode).await.expect("init clients error");
            for client in client_list.iter_mut() {
                client.flush_nodes(()).await?;
            }
            //打印信息
            return Ok(Response::new(entry));
        }
        Err(Status::new(Code::Unknown, "未知错误"))
    }
    async fn list_all_nodes(&self, request: Request<QueryNodeReq>) -> Result<Response<ListAllNodesResponse>, Status> {
        let req = request.into_inner();

        if req.node_type == NodeType::GroupNode as i32 {
            let nodes: Vec<NodeInfo> = self.shard_nodes.iter().map(|entry| entry.value().clone()).collect();
            let mut group_nodes: Vec<NodeInfo> = vec![];
            for entry in nodes.iter() {
                if entry.node_type == NodeType::GroupNode as i32 {
                    group_nodes.push(entry.clone());
                }
            }

            log::info!("获取所有节点信息");
            let response = ListAllNodesResponse {
                nodes: group_nodes,
            };
            return Ok(Response::new(response));
        }
        if req.node_type == NodeType::SocketNode as i32 {
            let nodes: Vec<NodeInfo> = self.socket_nodes.iter().map(|entry| entry.value().clone()).collect();
            let mut socket_nodes: Vec<NodeInfo> = vec![];
            for entry in nodes.iter() {
                if entry.node_type == NodeType::SocketNode as i32 {
                    socket_nodes.push(entry.clone());
                }
            }
            log::info!("获取所有节点信息");
            let response = ListAllNodesResponse {
                nodes: socket_nodes,
            };
            return Ok(Response::new(response));
        }
        let mut all_nodes: Vec<NodeInfo> = vec![];
        let nodes: Vec<NodeInfo> = self.socket_nodes.iter().map(|entry| entry.value().clone()).collect();
        all_nodes.extend(nodes);
        let nodes: Vec<NodeInfo> = self.shard_nodes.iter().map(|entry| entry.value().clone()).collect();
        all_nodes.extend(nodes);
        log::info!("获取所有节点信息");
        let response = ListAllNodesResponse {
            nodes: all_nodes,
        };
        return Ok(Response::new(response));
    }

    async fn graceful_leave(&self, request: Request<BaseRequest>) -> Result<Response<CommonResp>, Status> {
        let req = request.into_inner();
        let node_addr = req.node_addr;

        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;

        match self.shard_nodes.get_mut(&node_addr) {
            Some(mut entry) => {
                entry.state = ShardState::PreparingOffline as i32;
                entry.last_update_time = now;
                self.shard_nodes.remove(&node_addr);

                log::info!("节点 {} 已离线", node_addr);

                Ok(Response::new(CommonResp {
                    success: true,
                    message: format!("Node {} has gracefully left", node_addr),
                }))
            }
            None => Ok(Response::new(CommonResp {
                success: false,
                message: format!("Node {} not found", node_addr),
            })),
        }
    }

    async fn heartbeat(&self, request: Request<BaseRequest>) -> Result<Response<CommonResp>, Status> {
        match self.shard_nodes.get_mut(&request.get_ref().node_addr) {
            Some(mut entry) => {
                let value: &mut NodeInfo = entry.value_mut();
                // 更新心跳时间
                value.last_update_time = now() as u64;
                //打印日志
                log::info!("心跳: {}", &request.get_ref().node_addr);
                Ok(Response::new(CommonResp {
                    success: true,
                    message: "".to_string(),
                }))
            }
            None => Ok(Response::new(CommonResp {
                success: false,
                message: format!("Node {} not found", &request.get_ref().node_addr),
            })),
        }
    }
}
