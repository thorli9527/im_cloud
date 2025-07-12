//! ArbiterService - 基于内存状态的仲裁服务实现
//!
//! 本模块实现一个基于内存状态的仲裁服务 ArbiterServiceImpl，使用 RwLock + Arc 进行多线程访问保护，
//! 实现仲裁协议中的关键接口：节点注册、状态上报、分片信息查询、节点移除、版本更新等。
//!
//! 数据结构：
//! - ShardNodeInfoEntry：表示某 vnode 的状态信息
//! - NetworkNodeEntry：表示某物理节点的心跳和所拥有的 vnode 集合
//! - ArbiterState：维护 shard 和 node 的完整状态
//!
//! 接口说明：
//! - get_node：查询指定 vnode 的状态信息
//! - update_owner：通过 CAS 更新 vnode 的 owner 地址
//! - update_node_state：通过 CAS 更新 vnode 状态（如 Active、Leave）
//! - register_node：注册物理节点，记录其拥有 vnode 列表
//! - node_heartbeat：心跳接口，更新节点活跃时间与 vnode 状态
//! - is_local_node：判断某 vnode 是否归属当前节点
//! - list_nodes：列出所有节点及其 vnode 集合
//! - graceful_leave：节点优雅退出（仅清理记录）
//! - remove_node：强制剔除节点记录
//! - get_node_info：返回某节点的仲裁视图（vnode 分布）
//! - list_node：批量查询 vnode 状态

use chrono::Utc;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use dashmap::DashMap;
use tonic::{Request, Response, Status};
use tracing::log;
use common::util::date_util::now;
use crate::protocol::rpc_arb_models::{BaseRequest, CommonResp, ListAllNodesResponse, ShardNodeInfo, ShardState, UpdateShardStateRequest};
use crate::protocol::rpc_arb_server::arb_server_rpc_service_server::ArbServerRpcService;

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
                index: node_info.index,
                total: self.shard_nodes.len() as i32,
                version: node_info.version,
                state: node_info.state as i32,
                last_update_time: node_info.last_update_time,
            }));
        }


        // 获取所有已使用的 index
        let used_indices: HashSet<i32> = self
            .shard_nodes
            .iter()
            .map(|entry| entry.value().index)
            .collect();

        // 分配最小未使用 index
        let mut index = 0i32;
        while used_indices.contains(&index) {
            index += 1;
        }

        // 当前时间戳（毫秒）
        let now = now() as u64;
        // 构建新 entry
        let entry = ShardNodeInfo {
            node_addr: node_addr.clone(),
            index,
            total: self.shard_nodes.len() as i32,
            version: 0,
            state: ShardState::Preparing as i32,
            last_update_time: now,
        };

        // 插入
        self.shard_nodes.insert(node_addr.clone(), entry.clone());
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

