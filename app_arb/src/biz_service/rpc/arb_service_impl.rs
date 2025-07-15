
use crate::protocol::common::CommonResp;
use crate::protocol::rpc_arb_models::{BaseRequest, ListAllNodesResponse, ShardNodeInfo, ShardState, UpdateShardStateRequest};
use crate::protocol::rpc_arb_server::arb_server_rpc_service_server::ArbServerRpcService;
use chrono::Utc;
use common::util::date_util::now;
use dashmap::DashMap;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use tonic::{Request, Response, Status};
use tracing::log;

/// 分片节点状态信息
/// 表示某个 vnode 当前的版本号、状态、归属节点及上次更新时间

/// ArbiterService 实现体，持有共享状态引用
#[derive(Clone, Default)]
pub struct ArbiterServiceImpl {
    pub shard_nodes: Arc<DashMap<String, ShardNodeInfo>>,
}

#[tonic::async_trait]
impl ArbServerRpcService for ArbiterServiceImpl {
    // === 分片管理 ===
    async fn get_shard_node(
        &self,
        req: Request<BaseRequest>,
    ) -> Result<Response<ShardNodeInfo>, Status> {
        let node_addr = req.into_inner().node_addr;

        match self.shard_nodes.get(&node_addr) {
            Some(entry) => {
                Ok(Response::new(entry.value().clone()))
            }
            None => Err(Status::not_found(format!(
                "Node {} not found.",
                node_addr
            ))),
        }
    }

    async fn update_shard_state(
        &self,
        request: Request<UpdateShardStateRequest>,
    ) -> Result<Response<CommonResp>, Status> {
        let req = request.into_inner();
        let node_addr = req.node_addr;
        let new_state = req.new_state;

        let mut shard_nodes = &self.shard_nodes;

        match shard_nodes.get_mut(&node_addr) {
            Some(mut entry) => {
                // 更新状态和元信息
                entry.state = match ShardState::from_i32(new_state) {
                    Some(valid_state) => valid_state as i32,
                    None => {
                        return Err(Status::invalid_argument(format!(
                            "Invalid ShardState: {}",
                            new_state
                        )))
                    }
                };
                entry.version += 1;
                entry.last_update_time = now() as u64;

                Ok(Response::new(CommonResp {
                    success: true,
                    message: format!(
                        "Updated node {} to state {:?}, version = {}",
                        node_addr, entry.state, entry.version
                    ),
                }))
            }

            None => Err(Status::not_found(format!(
                "Node {} not found.",
                node_addr
            ))),
        }
    }

    /// 注册节点：如果 node_addr 已存在，则返回失败；否则分配唯一 index 并插入
    async fn register_node(
        &self,
        request: Request<BaseRequest>,
    ) -> Result<Response<ShardNodeInfo>, Status> {
        let req = request.into_inner();
        let node_addr = req.node_addr;

        // 如果已存在，返回已有信息
        if let Some(node_info) = self.shard_nodes.get(&node_addr) {
            return Ok(Response::new(crate::protocol::rpc_arb_models::ShardNodeInfo {
                node_addr: node_info.node_addr.clone(),
                total: self.shard_nodes.len() as i32,
                version: node_info.version,
                state: node_info.state as i32,
                last_update_time: node_info.last_update_time,
            }));
        }

        // 当前时间戳（毫秒）
        let now = now() as u64;
        // 构建新 entry
        let mut  entry = ShardNodeInfo {
            node_addr: node_addr.clone(),
            total: self.shard_nodes.len() as i32,
            version: 0,
            state: ShardState::Preparing as i32,
            last_update_time: now,
        };

        // 插入
        self.shard_nodes.insert(node_addr.clone(), entry.clone());
        entry.total = self.shard_nodes.len() as i32;
        //打印信息
        // log::warn!("新增分片节点: {:?}", &entry);
        return Ok(Response::new(entry));
    }
    async fn list_all_nodes(
        &self,
        _request: Request<()>,
    ) -> Result<Response<ListAllNodesResponse>, Status> {
        let nodes: Vec<ShardNodeInfo> = self
            .shard_nodes
            .iter()
            .map(|entry| entry.value().clone())
            .collect();

        log::info!("获取所有节点信息");

        let response = ListAllNodesResponse { nodes };
        Ok(Response::new(response))
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

    async fn heartbeat(&self, request: Request<BaseRequest>) -> Result<Response<CommonResp>, Status> {
        match self.shard_nodes.get_mut(&request.get_ref().node_addr) {
            Some(mut entry) => {
                let value: &mut ShardNodeInfo = entry.value_mut();
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

