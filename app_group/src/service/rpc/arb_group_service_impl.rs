use std::net::SocketAddr;
use std::str::FromStr;
use crate::manager::shard_manager;
use crate::manager::shard_manager::ShardManager;
use crate::protocol::rpc_arb_group;
use crate::protocol::rpc_arb_group::UpdateVersionReq;
use crate::protocol::rpc_arb_models::{BaseRequest, ShardNodeInfo, SyncListGroup};
use biz_service::protocol::common::CommonResp;
use common::util::common_utils::hash_index;
use common::util::date_util::now;
use log::info;
use std::thread::current;
use tonic::{Request, Response, Status};
use common::config::AppConfig;
use crate::protocol::rpc_arb_group::arb_group_service_server::ArbGroupServiceServer;

/// arb 组 客户端接口
pub struct ArbGroupServiceImpl {
}
impl ArbGroupServiceImpl{
    pub async fn start(&self) {
        // 读取配置文件
        let app_cfg = AppConfig::get();
        let addr = SocketAddr::from_str(&app_cfg.get_shard().server_host.unwrap()).expect("Invalid address");
        let svc = ArbGroupServiceImpl {};
        tonic::transport::Server::builder()
            .add_service(ArbGroupServiceServer::new(svc))
            .serve(addr)
            .await
            .expect("Failed to start server");
        log::warn!("ArbGroupServiceServer started");
    }
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
            node_type:req.node_type,
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

    async fn sync_data(&self, request: Request<SyncListGroup>) -> Result<Response<CommonResp>, Status> {
        let  shard_manager = ShardManager::get();
        let list=request.into_inner();
        if !list.groups.is_empty() {
            for group in list.groups.iter() {
                shard_manager.add_group(&group).await;;
            }
        }
        if !list.members.is_empty() {
            for member in list.members.iter() {
                shard_manager.add_user_to_group(&member.group_id, &member.uid);
            }
        }
        return Ok(Response::new(CommonResp {
            success: true,
            message: format!("Sync {} groups", list.groups.len()),
        }));
    }
}