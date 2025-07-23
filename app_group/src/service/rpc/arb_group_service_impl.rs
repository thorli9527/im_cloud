use crate::manager::shard_manager;
use crate::manager::shard_manager::{ShardManager, ShardManagerMqOpt, ShardManagerOpt};
use biz_service::protocol::arb::rpc_arb_group;
use biz_service::protocol::arb::rpc_arb_group::UpdateVersionReq;
use biz_service::protocol::arb::rpc_arb_group::arb_group_service_server::ArbGroupServiceServer;
use biz_service::protocol::arb::rpc_arb_models::SyncListGroup;
use biz_service::protocol::common::CommonResp;
use common::config::AppConfig;
use common::util::common_utils::hash_index;
use common::util::date_util::now;
use log::info;
use std::net::SocketAddr;
use std::str::FromStr;
use tonic::{Request, Response, Status};

/// arb 组 客户端接口
pub struct ArbGroupServiceImpl {}
impl ArbGroupServiceImpl {
    pub async fn start(&self) {
        // 读取配置文件
        let app_cfg = AppConfig::get();
        let addr = SocketAddr::from_str(&app_cfg.get_shard().server_host.unwrap())
            .expect("Invalid address");
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
            current.version = req.version;
            let time = now() as u64;
            let i = hash_index(req.node_addr.as_str(), req.total);
            current.index = i;
            current.total = req.total;
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
        let list = request.into_inner();
        let mut group_errors = Vec::new();
        let mut member_errors = Vec::new();

        // 处理群组同步
        for group in &list.groups {
            if let Err(e) = shard_manager.create_group(group).await {
                group_errors.push(format!("group_id={}: {}", group, e));
            }
        }

        // 处理成员同步
        for member in &list.members {
            if let Err(e) = shard_manager.add_user_to_group(&member.group_id, &member.uid) {
                member_errors
                    .push(format!("group_id={}, uid={}: {}", member.group_id, member.uid, e));
            }
        }

        // 构造响应
        if group_errors.is_empty() && member_errors.is_empty() {
            Ok(Response::new(CommonResp {
                success: true,
                message: format!(
                    "Sync success: {} groups, {} members",
                    list.groups.len(),
                    list.members.len()
                ),
            }))
        } else {
            let mut all_errors = group_errors;
            all_errors.extend(member_errors);
            Ok(Response::new(CommonResp {
                success: false,
                message: format!("Sync failed:\n{}", all_errors.join("\n")),
            }))
        }
    }
}
