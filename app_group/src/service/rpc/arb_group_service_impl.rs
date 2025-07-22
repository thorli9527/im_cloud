use crate::manager::shard_manager;
use crate::manager::shard_manager::{ShardManager, ShardManagerMqOpt, ShardManagerOpt};
use biz_service::protocol::common::CommonResp;
use common::config::AppConfig;
use common::util::common_utils::hash_index;
use common::util::date_util::now;
use log::info;
use std::net::SocketAddr;
use std::str::FromStr;
use std::thread::current;
use tonic::{Request, Response, Status};
use biz_service::protocol::arb::rpc_arb_group;
use biz_service::protocol::arb::rpc_arb_group::arb_group_service_server::ArbGroupServiceServer;
use biz_service::protocol::arb::rpc_arb_group::UpdateVersionReq;
use biz_service::protocol::arb::rpc_arb_models::SyncListGroup;

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
                shard_manager.create_group(group).await;;
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