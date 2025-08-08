use crate::service::shard_manager::ShardManager;
use common::config::AppConfig;
use common::util::common_utils::hash_index;
use common::util::date_util::now;
use log::info;
use std::net::SocketAddr;
use std::str::FromStr;
use tonic::{Request, Response, Status};
use biz_service::kafka_util::node_util::NodeUtil;
use biz_service::protocol::arb::arb_client::arb_client_service_server::ArbClientService;
use biz_service::protocol::arb::arb_client::UpdateVersionReq;
use biz_service::protocol::arb::arb_models::SyncListGroup;
use biz_service::protocol::common::CommonResp;

/// arb 组 客户端接口
pub struct ArbClientServiceImpl {}
impl ArbClientServiceImpl {
    pub fn new() -> Self {
        Self {}
    }
}
#[tonic::async_trait]
impl ArbClientService for ArbClientServiceImpl {
    async fn update_version(
        &self,
        request: Request<UpdateVersionReq>,
    ) -> Result<Response<CommonResp>, Status> {
        let req = request.into_inner();
        let shard_manager = ShardManager::get();

        {
            // 加写锁并更新 version 字段
            let guard = shard_manager.current.load();
            let mut current = guard.shard_info.write().await;
            let time = now() as u64;
            let i = hash_index(req.node_addr.as_str(), req.total);
            current.last_update_time = time;
            info!("🔄 版本更新成功: 新版本={} 更新时间={}", req.version, time);
        }

        Ok(Response::new(CommonResp {
            success: true,
            message: format!("Version updated to {}", req.version),
        }))
    }

    async fn sync_data(
        &self,
        request: Request<SyncListGroup>,
    ) -> Result<Response<CommonResp>, Status> {
        let shard_manager = ShardManager::get();
        let sync_data = request.into_inner();

        let guard = shard_manager.current.load();
        let member_len = sync_data.members.len();
        guard.shard_map.insert_many(&sync_data.group_id, sync_data.members);
        sync_data.on_line_ids.iter().for_each(|user_id| {
            guard.shard_map.set_online(&sync_data.group_id, user_id, true);
        });
        Ok(Response::new(CommonResp {
            success: true,
            message: format!("Sync success: {} groups, {} members", sync_data.group_id, member_len),
        }))
    }

    async fn flush_nodes(&self, request: Request<()>) -> Result<Response<CommonResp>, Status> {
        let node_util = NodeUtil::get();

        Ok(Response::new(CommonResp {
            success: true,
            message: format!("Sync success"),
        }))
    }
}
