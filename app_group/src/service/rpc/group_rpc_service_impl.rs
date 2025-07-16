use crate::manager::shard_manager::ShardManager;
use crate::protocol::rpc_arb_group::arb_group_service_server::ArbGroupServiceServer;
use crate::protocol::rpc_group_models::{
    AddMemberReq, ChangeMemberAliasReq, ChangeMemberRoleReq, CreateGroupReq, DestroyGroupReq,
    ExitGroupReq, GetGroupInfoRep, GetGroupInfoReq, GetMembersRep, GetMembersReq,
    GetOnlineMembersReq, GetOnlineMembersResp, MuteMemberReq, OfflineReq, OnlineReq,
    RemoveMemberReq, TransferGroupOwnershipReq, UpdateGroupInfoReq,
};
use crate::protocol::rpc_group_server::group_rpc_service_server::{GroupRpcService, GroupRpcServiceServer};
use crate::service::rpc::arb_group_service_impl::ArbGroupServiceImpl;
use biz_service::biz_service::group_service::GroupService;
use biz_service::manager::group_manager_core::{GroupManager, GroupManagerOpt};
use biz_service::protocol::common::{CommonResp, GroupEntity, GroupRoleType, GroupType};
use common::config::AppConfig;
use common::util::date_util::now;
use once_cell::sync::OnceCell;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use tonic::{Request, Response, Status};

/// 群聊服务端rpc服务
pub struct GroupRpcServiceImpl {}

impl GroupRpcServiceImpl {
    pub async fn start(&self) {
        // 读取配置文件
        let app_cfg = AppConfig::get();
        let addr = SocketAddr::from_str(&app_cfg.get_shard().server_host.unwrap()).expect("Invalid address");
        let svc = GroupRpcServiceImpl {};

        tonic::transport::Server::builder()
            .add_service(GroupRpcServiceServer::new(svc))
            .serve(addr)
            .await
            .expect("Failed to start server");
        log::warn!("GroupRpcServiceServer started");
    }
    pub fn new() -> Self {
        Self {}
    }

    pub async fn hash_group_id(&self, group_id: &str) -> usize {
        let shard_manager = ShardManager::get();
        let guard = shard_manager.current.load();
        let shard_info = guard.shard_info.write().await;
        if shard_info.total == 0 {
            return 0; // 如果没有分片，返回0
        }
        use std::hash::{Hash, Hasher};
        use twox_hash::XxHash64;

        let mut hasher = XxHash64::with_seed(0);
        group_id.hash(&mut hasher);
        (hasher.finish() as usize) % shard_info.total as usize
    }
    /// 获取单例
    pub fn get() -> Arc<Self> {
        INSTANCE_COUNTRY
            .get()
            .expect("INSTANCE is not initialized")
            .clone()
    }
}
static INSTANCE_COUNTRY: OnceCell<Arc<GroupRpcServiceImpl>> = OnceCell::new();
#[tonic::async_trait]
impl GroupRpcService for GroupRpcServiceImpl {
    async fn online(&self, request: Request<OnlineReq>) -> Result<Response<CommonResp>, Status> {
        let shard_manager = ShardManager::get();
        let req = request.get_ref();
        let shard_id = self.hash_group_id(&req.group_id).await;
        if shard_id == 0 {
            return Err(Status::not_found("Shard.error"));
        }
        let guard = shard_manager.current.load();
        let current = guard.shard_info.write().await;
        if shard_id != current.index as usize {
            return Err(Status::not_found("Shard.error"));
        }
        shard_manager.mark_user_online(&req.group_id, &req.uid);
        Ok(Response::new(CommonResp {
            success: true,
            message: "".to_string(),
        }))
    }

    async fn offline(&self, request: Request<OfflineReq>) -> Result<Response<CommonResp>, Status> {
        let shard_manager = ShardManager::get();
        let req = request.get_ref();
        shard_manager.mark_user_offline(&req.group_id, &req.uid);
        Ok(Response::new(CommonResp {
            success: true,
            message: "".to_string(),
        }))
    }

    async fn get_online_members(
        &self,
        request: Request<GetOnlineMembersReq>,
    ) -> Result<Response<GetOnlineMembersResp>, Status> {
        let shard_manager = ShardManager::get();
        let req = request.get_ref();
        let user_ids = shard_manager.get_online_users_for_group(&req.group_id);
        Ok(Response::new(GetOnlineMembersResp {
            total_count: user_ids.len() as i32,
            uids: user_ids,
            success: true,
            message: "".to_string(),
        }))
    }

    async fn get_members(
        &self,
        request: Request<GetMembersReq>,
    ) -> Result<Response<GetMembersRep>, Status> {
        let req = request.get_ref();

        let page = req.page.max(1);
        let page_size = req.page_size.max(1);
        let offset = ((page - 1) as usize) * (page_size as usize);
        let limit = page_size as usize;

        let shard_manager = ShardManager::get();

        // 获取总成员数（提前，避免超界分页）
        let total_count = shard_manager
            .get_group_member_total_count(&req.group_id)
            .unwrap_or_default();

        // 如果 offset 已超出成员总数，返回空页
        let user_ids = if offset >= total_count {
            vec![]
        } else {
            shard_manager.get_group_members_page(&req.group_id, offset, limit)
        };

        let response = GetMembersRep {
            uids: user_ids,
            total_count: total_count as i32,
            success: true,
            message: "".to_string(),
        };

        Ok(Response::new(response))
    }

    async fn create_group(
        &self,
        request: Request<CreateGroupReq>,
    ) -> Result<Response<CommonResp>, Status> {
        let group_manager = GroupManager::get();

        let req = request.into_inner();
        let group_info = GroupEntity {
            id: req.id,
            name: req.name,
            avatar: req.avatar,
            description: req.description,
            notice: "".to_string(),
            owner_id: req.owner_id,
            group_type: GroupType::NormalGroup as i32,
            join_permission: Default::default(),
            allow_search: false,
            enable: true,
            create_time: 0,
            update_time: 0,
        };
        let resp = group_manager.create_group(&group_info).await;
        match resp {
            Ok(_) => Ok(Response::new(CommonResp {
                success: true,
                message: "Group created successfully".to_string(),
            })),
            Err(e) => Err(Status::internal(format!("Failed to create group: {}", e))),
        }
    }

    async fn destroy_group(
        &self,
        request: Request<DestroyGroupReq>,
    ) -> Result<Response<CommonResp>, Status> {
        let group_manager = GroupManager::get();
        let req = request.into_inner();
        let resp = group_manager.dismiss_group(&req.group_id).await;
        match resp {
            Ok(_) => Ok(Response::new(CommonResp {
                success: true,
                message: "Group destroyed successfully".to_string(),
            })),
            Err(e) => Err(Status::internal(format!("Failed to destroy group: {}", e))),
        }
    }

    async fn get_group_info(
        &self,
        request: Request<GetGroupInfoReq>,
    ) -> Result<Response<GetGroupInfoRep>, Status> {
        let group_manager = GroupManager::get();
        let req = request.into_inner();
        let group_info = group_manager
            .get_group_info(&req.group_id)
            .await
            .expect("Group not found");
        if group_info.is_none() {
            return Err(Status::not_found("Group not found"));
        }
        let group = group_info.unwrap();
        Ok(Response::new(GetGroupInfoRep {
            group_id: group.id,
            name: group.name,
            avatar: group.avatar,
            description: group.description,
            owner_id: group.owner_id,
            members: vec![],
        }))
    }

    async fn update_group_info(
        &self,
        request: Request<UpdateGroupInfoReq>,
    ) -> Result<Response<CommonResp>, Status> {
        let group_service = GroupService::get();

        let req = request.into_inner();

        let group_manager = GroupManager::get();

        let group = group_service
            .find_by_group_id(&req.id)
            .await
            .expect("Group not found");
        let group_info = GroupEntity {
            id: req.id,
            name: req.name,
            avatar: req.avatar,
            description: req.description,
            notice: req.notice,
            owner_id: group.owner_id, // 这里可以根据需要设置
            group_type: req.group_type,
            join_permission: req.join_permission,
            allow_search: req.allow_search,
            enable: group.enable,
            create_time: group.create_time, // 这里可以根据需要设置
            update_time: now() as u64,      // 这里可以根据需要设置
        };
        group_manager
            .update_group(&group_info)
            .await
            .expect("Failed to update group info");
        Ok(Response::new(CommonResp {
            success: true,
            message: "Group info updated successfully".to_string(),
        }))
    }

    async fn add_member(
        &self,
        request: Request<AddMemberReq>,
    ) -> Result<Response<CommonResp>, Status> {
        let group_manager = GroupManager::get();
        let req = request.into_inner();
        //循环members 并添加
        for member in req.members {
            let option = GroupRoleType::from_i32(member.role);
            group_manager
                .add_user_to_group(&req.group_id, &member.uid, &member.alias, &option.unwrap())
                .await
                .expect("add group member error");
        }

        Ok(Response::new(CommonResp {
            success: true,
            message: "所有成员添加成功".into(),
        }))
    }

    async fn remove_member(
        &self,
        request: Request<RemoveMemberReq>,
    ) -> Result<Response<CommonResp>, Status> {
        let group_manager = GroupManager::get();
        let req = request.into_inner();
        for uid in req.uids {
            group_manager
                .remove_user_from_group(&req.group_id, &uid)
                .await
                .expect("remove group member error");
        }
        Ok(Response::new(CommonResp {
            success: true,
            message: "移除成功".into(),
        }))
    }

    async fn exit_group(
        &self,
        request: Request<ExitGroupReq>,
    ) -> Result<Response<CommonResp>, Status> {
        let group_manager = GroupManager::get();
        let req = request.into_inner();
        group_manager
            .remove_user_from_group(&req.group_id, &req.uid)
            .await
            .expect("remove group member error");
        Ok(Response::new(CommonResp {
            success: true,
            message: "退出成功".into(),
        }))
    }

    async fn change_member_role(
        &self,
        request: Request<ChangeMemberRoleReq>,
    ) -> Result<Response<CommonResp>, Status> {
        let group_manager = GroupManager::get();
        let req = request.into_inner();
        let group_role_type = GroupRoleType::from_i32(req.role).expect("role is invalid");
        group_manager
            .change_member_role(&req.group_id, &req.uid, &group_role_type)
            .await
            .expect("change group member role error");
        Ok(Response::new(CommonResp {
            success: true,
            message: "".to_string(),
        }))
    }

    async fn mute_member(
        &self,
        request: Request<MuteMemberReq>,
    ) -> Result<Response<CommonResp>, Status> {
        todo!()
    }

    async fn change_member_alias(
        &self,
        request: Request<ChangeMemberAliasReq>,
    ) -> Result<Response<CommonResp>, Status> {
        todo!()
    }

    async fn transfer_ownership(
        &self,
        request: Request<TransferGroupOwnershipReq>,
    ) -> Result<Response<CommonResp>, Status> {
        todo!()
    }
}
