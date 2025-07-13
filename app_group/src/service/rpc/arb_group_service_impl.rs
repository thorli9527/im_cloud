use crate::manager::shard_manager;
use crate::manager::shard_manager::ShardManager;
use crate::protocol::common::CommonResp;
use crate::protocol::rpc_arb_group;
use crate::protocol::rpc_arb_group::UpdateVersionReq;
use crate::protocol::rpc_arb_models::{BaseRequest, ShardNodeInfo, SyncDataReq};
use common::util::common_utils::hash_index;
use common::util::date_util::now;
use log::info;
use std::thread::current;
use tonic::{Request, Response, Status};

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
        let req=_request.into_inner();
        // 只读取一次锁，避免重复锁开销
        let guard = shard_manager.current.load();
        let shard_info = guard.shard_info.write().await;
        let info = ShardNodeInfo {
            node_addr: shard_manager.get_node_addr().to_string(),
            version: shard_info.version,
            state: shard_info.state as i32,
            last_update_time: shard_info.last_update_time,
            total:shard_info.total,
        };
        Ok(Response::new(info))
    }

    async fn update_version(&self, request: Request<UpdateVersionReq>) -> Result<Response<CommonResp>, Status> {
        let req = request.into_inner();
        let  shard_manager = ShardManager::get();

        {
            // 加写锁并更新 version 字段
            let guard = shard_manager.current.load();
            let mut current = guard.shard_info.write().await;
            current.version = req.version;
            let time = now() as u64;
            let i = hash_index(req.node_addr.as_str(), req.total);
            current.index=i;
            current.total = req.total;
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

    async fn sync_data(&self, request: Request<SyncDataReq>) -> Result<Response<CommonResp>, Status> {
        todo!()
    }
}