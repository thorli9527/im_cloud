use std::thread::current;
use log::info;
use tonic::{Request, Response, Status};
use common::util::date_util::now;
use crate::manager::shard_manager;
use crate::manager::shard_manager::ShardManager;
use crate::protocol::rpc_arb_group;
use crate::protocol::rpc_arb_group::UpdateVersionReq;
use crate::protocol::rpc_arb_models::{BaseRequest, CommonResp, ShardNodeInfo};

/// arb 组 客户端接口
pub struct ArbGroupServiceImpl {
}
#[tonic::async_trait]
impl rpc_arb_group::arb_group_service_server::ArbGroupService for ArbGroupServiceImpl {
    async fn get_shard_node(
        &self,
        _request: Request<BaseRequest>,
    ) -> Result<Response<ShardNodeInfo>, Status> {
        let shard_manager = ShardManager::get();
        // 只读取一次锁，避免重复锁开销
        let current = shard_manager.current.read().await;
        let info = ShardNodeInfo {
            node_addr: shard_manager.get_node_addr().to_string(),
            version: current.version,
            state: current.state as i32,
            last_update_time: current.last_update_time,
            index:shard_manager.index,
            total:shard_manager.total,
        };

        Ok(Response::new(info))
    }

    async fn update_version(&self, request: Request<UpdateVersionReq>) -> Result<Response<crate::protocol::rpc_arb_models::CommonResp>, Status> {
        let req = request.into_inner();
        let shard_manager = ShardManager::get();

        {
            // 加写锁并更新 version 字段
            let mut current = shard_manager.current.write().await;
            current.version = req.version;
            let time = now() as u64;
            current.last_update_time = time;
            info!(
                "🔄 版本更新成功: 新版本={} 更新时间={}",
                req.version, time
            );
        }

        Ok(Response::new(CommonResp {
            success: true,
            message: format!("Version updated to {}", req.version),
        }))
    }
}