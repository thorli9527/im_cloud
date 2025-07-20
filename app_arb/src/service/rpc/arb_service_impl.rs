use crate::protocol::common::CommonResp;
use crate::protocol::rpc_arb_group::arb_group_service_client::ArbGroupServiceClient;
use crate::protocol::rpc_arb_group::UpdateVersionReq;
use crate::protocol::rpc_arb_models::{BaseRequest, ListAllNodesResponse, NodeInfo, NodeType, QueryNodeReq, ShardState, UpdateShardStateRequest};
use crate::protocol::rpc_arb_server::arb_server_rpc_service_server::ArbServerRpcService;
use crate::protocol::rpc_arb_socket::arb_socket_service_client::ArbSocketServiceClient;
use common::util::date_util::now;
use dashmap::DashMap;
use std::clone;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tonic::transport::Channel;
use tonic::{Request, Response, Status};
use tracing::log;

/// 分片节点状态信息
/// 表示某个 vnode 当前的版本号、状态、归属节点及上次更新时间

/// ArbiterService 实现体，持有共享状态引用
#[derive(Clone, Default)]
pub struct ArbiterServiceImpl {
    pub shard_nodes: Arc<DashMap<String, NodeInfo>>,
    pub socket_nodes: Arc<DashMap<String, NodeInfo>>,
    pub arb_version: Arc<AtomicU64>,
}


impl ArbiterServiceImpl {

    pub async fn init_socket_clients(&self) -> Result<Vec<ArbSocketServiceClient<Channel>>, Box<dyn std::error::Error + Send + Sync>> {
        let mut endpoints=Vec::new();
        for ref node in self.socket_nodes.iter() {
            let node_info = node.value();
            endpoints.push(node_info.node_addr.clone());
        }
        let mut clients = Vec::new();
        for endpoint in endpoints {
            let channel = Channel::from_shared(format!("http://{}",endpoint.clone()))?.connect().await?;
            let client = ArbSocketServiceClient::new(channel);
            clients.push(client);
        }
        Ok(clients)
    }
    pub async fn init_shard_clients(&self) -> Result<Vec<ArbGroupServiceClient<Channel>>, Box<dyn std::error::Error + Send + Sync>> {
        let mut endpoints=Vec::new();
        for ref node in self.shard_nodes.iter() {
            let node_info = node.value();
            endpoints.push(node_info.node_addr.clone());
        }
        let mut clients = Vec::new();
        for endpoint in endpoints {
            let channel = Channel::from_shared(format!("http://{}",endpoint.clone()))?.connect().await?;
            let client = ArbGroupServiceClient::new(channel);
            clients.push(client);
        }
        Ok(clients)
    }

}

#[tonic::async_trait]
impl ArbServerRpcService for ArbiterServiceImpl {
    // === 分片管理 ===

    async fn update_shard_state(
        &self,
        request: Request<UpdateShardStateRequest>,
    ) -> Result<Response<CommonResp>, Status> {
        let req = request.into_inner();
        let current_node_addr = req.node_addr.clone();
        let new_state = req.new_state;
        
        // 更新目标节点状态
        match self.shard_nodes.get_mut(&current_node_addr) {
            Some(mut entry) => {
                entry.state = match ShardState::from_i32(new_state) {
                    Some(valid_state) => valid_state as i32,
                    None => {
                        return Err(Status::invalid_argument(format!(
                            "Invalid ShardState: {}",
                            new_state
                        )));
                    }
                };
                entry.version += 1;
                entry.last_update_time = now() as u64;
                if entry.node_type == NodeType::GroupNode as i32 {
                    entry.total = self.shard_nodes.len() as i32;
                  
                    // 如果新状态是 Normal，检查所有 shard 状态
                    if req.new_state == ShardState::Normal as i32 {
                        // 统计是否所有 shard 节点状态为 Normal
                        let all_normal = self.shard_nodes.iter().all(|entry| {
                            entry.value().state == ShardState::Normal as i32
                            
                        });

                        if all_normal {
                            // 升级 arb_version
                            let new_version = self.arb_version.fetch_add(1, Ordering::SeqCst) + 1;

                            log::info!(
                                "[ArbVersion] All shard nodes are Normal. Upgrading arb_version to {}",
                                new_version
                            );

                            // 通知所有 socket 节点刷新 shard 列表
                            if let Ok(mut socket_clients) = self.init_socket_clients().await {
                                for client in socket_clients.iter_mut() {
                                    let _ = client.flush_shard_list(Request::new(())).await;
                                }
                            }
                        } else {
                            log::info!(
                                "[ArbVersion] Node {} is Normal, but not all are Normal yet.",
                                current_node_addr
                            );
                        }
                    }
                }

                Ok(Response::new(CommonResp {
                    success: true,
                    message: format!(
                        "Updated node {} to state {:?}, version = {}",
                        current_node_addr, entry.state, entry.version
                    ),
                }))
            }

            None => Err(Status::not_found(format!("Node {} not found.", current_node_addr))),
        }
    }


    /// 注册节点：如果 node_addr 已存在，则返回失败；否则分配唯一 index 并插入
    async fn register_node(
        &self,
        request: Request<BaseRequest>,
    ) -> Result<Response<NodeInfo>, Status> {
        let req = request.into_inner();
        let node_addr = req.node_addr;
        if req.node_type == NodeType::GroupNode as i32 {
            // 如果已存在，返回已有信息
            if let Some(node_info) = self.shard_nodes.get(&node_addr) {
                return Ok(Response::new(
                    crate::protocol::rpc_arb_models::NodeInfo {
                        node_addr: node_info.node_addr.clone(),
                        total: self.shard_nodes.len() as i32,
                        version: node_info.version,
                        state: node_info.state,
                        node_type: req.node_type,
                        last_update_time: node_info.last_update_time,
                        kafka_addr: req.kafka_addr,
                        socket_addr:None
                    },
                ));
            }
            let current_total = self.shard_nodes.len() as i32 + 1;
            // 当前时间戳（毫秒）
            let now = now() as u64;
            // 构建新 entry
            let mut entry = NodeInfo {
                node_addr: node_addr.clone(),
                total: current_total,
                version: 0,
                node_type: req.node_type,
                state: ShardState::Preparing as i32,
                last_update_time: now,
                kafka_addr: req.kafka_addr,
                socket_addr:None
            };
           
            for (mut item) in self.shard_nodes.iter_mut() {
                item.state=ShardState::Preparing as i32;
                let mut client_list =self.init_shard_clients().await.expect("init shard clients error");
                for mut client in client_list.iter_mut() {
                    let request=Request::new(UpdateVersionReq{
                        node_addr: item.node_addr.clone(),
                        version: item.version,
                        state: item.state,
                        last_update_time: now,
                        total: current_total,
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
        } else {
            // 如果已存在，返回已有信息
            if let Some(node_info) = self.socket_nodes.get(&node_addr) {
                return Ok(Response::new(
                    crate::protocol::rpc_arb_models::NodeInfo {
                        node_addr: node_info.node_addr.clone(),
                        total: self.socket_nodes.len() as i32,
                        version: node_info.version,
                        state: node_info.state,
                        node_type: req.node_type,
                        last_update_time: node_info.last_update_time,
                        kafka_addr: req.kafka_addr,
                        socket_addr:None
                    },
                ));
            }

            // 当前时间戳（毫秒）
            let now = now() as u64;
            // 构建新 entry
            let mut entry = NodeInfo {
                node_addr: node_addr.clone(),
                total: self.shard_nodes.len() as i32,
                version: 0,
                node_type: req.node_type,
                state: ShardState::Preparing as i32,
                last_update_time: now,
                kafka_addr: req.kafka_addr,
                socket_addr:None
            };

            // 插入
            self.socket_nodes.insert(node_addr.clone(), entry.clone());
            entry.total = self.socket_nodes.len() as i32;
            //打印信息
            // log::warn!("新增分片节点: {:?}", &entry);
            return Ok(Response::new(entry));
        }
    }
    async fn list_all_nodes(
        &self,
        request: Request<QueryNodeReq>,
    ) -> Result<Response<ListAllNodesResponse>, Status> {
        let req = request.into_inner();
        if req.node_type == NodeType::GroupNode as i32 {
            let nodes: Vec<NodeInfo> = self
                .shard_nodes
                .iter()
                .map(|entry| entry.value().clone())
                .collect();

            log::info!("获取所有节点信息");
            let response = ListAllNodesResponse { nodes };
            return Ok(Response::new(response));
        }
        let nodes: Vec<NodeInfo> = self
            .socket_nodes
            .iter()
            .map(|entry| entry.value().clone())
            .collect();
        log::info!("获取所有节点信息");
        let response = ListAllNodesResponse { nodes };
        return Ok(Response::new(response));
    }

    async fn graceful_leave(
        &self,
        request: Request<BaseRequest>,
    ) -> Result<Response<CommonResp>, Status> {
        let req = request.into_inner();
        let node_addr = req.node_addr;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

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

    async fn heartbeat(
        &self,
        request: Request<BaseRequest>,
    ) -> Result<Response<CommonResp>, Status> {
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
