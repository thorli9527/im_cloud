use crate::manager::shard_manager;
use crate::manager::shard_manager::ShardManager;
use crate::protocol::rpc_group_models::{
    AddMemberReq, ChangeMemberAliasReq, ChangeMemberRoleReq, CommonResp, CreateGroupReq,
    DestroyGroupReq, ExitGroupReq, GetGroupInfoRep, GetGroupInfoReq, GetMembersRep, GetMembersReq,
    GetOnlineMembersReq, GetOnlineMembersResp, MuteMemberReq, OfflineReq, OnlineReq,
    RemoveMemberReq, TransferGroupOwnershipReq, UpdateGroupInfoReq,
};
use crate::protocol::rpc_group_server::group_rpc_service_server::GroupRpcService;
use actix_web::web::get;
use std::sync::Arc;
use tonic::{Request, Response, Status};
use common::UserId;

/// 群聊服务端rpc服务
struct GroupRpcServiceImpl {
    // 这里可以添加一些共享状态或依赖注入
}

impl GroupRpcServiceImpl {
    pub fn new() -> Self {
        Self {}
    }
    pub fn hash_group_id(&self, group_id: &str) -> usize {
        let shard_manager = ShardManager::get();
        if shard_manager.total == 0 {
            return 0; // 如果没有分片，返回0
        }
        use std::hash::{Hash, Hasher};
        use twox_hash::XxHash64;

        let mut hasher = XxHash64::with_seed(0);
        group_id.hash(&mut hasher);
        (hasher.finish() as usize) % shard_manager.total as usize
    }
}
#[tonic::async_trait]
impl GroupRpcService for GroupRpcServiceImpl {
    async fn online(&self, request: Request<OnlineReq>) -> Result<Response<CommonResp>, Status> {
        let shard_manager = ShardManager::get();
        let req = request.get_ref();
        let shard_id = self.hash_group_id(&req.group_id);
        if shard_id == 0 {
            return Err(Status::not_found("Shard.error"));
        }
        if shard_id != shard_manager.index as usize{
            return Err(Status::not_found("Shard.error"));
        }
        shard_manager.mark_user_online(&req.group_id,&req.uid);
        Ok(Response::new(CommonResp {
            success: true,
            message: "".to_string(),
        }))
    }

    async fn offline(&self, request: Request<OfflineReq>) -> Result<Response<CommonResp>, Status> {
        let shard_manager = ShardManager::get();
        let req = request.get_ref();
        shard_manager.mark_user_offline(&req.group_id,&req.uid);
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
        let user_ids=shard_manager.get_online_users_for_group(&req.group_id);
        Ok(Response::new(GetOnlineMembersResp {
            total_count: user_ids.len() as i32,
            uids:user_ids,
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

        // 🔹 分页获取成员 UID 列表
        let user_ids = shard_manager.get_group_members_page(&req.group_id, offset, limit);

        // 🔹 获取群成员总数（如有 total 接口，否则 fallback）
        let total_count = shard_manager
            .get_group_member_total_count(&req.group_id)
            .unwrap_or(user_ids.len() as i32); // fallback 逻辑

        let response = GetMembersRep {
            uids: user_ids,
            total_count,
            success: true,
            message: "".to_string(),
        };

        Ok(Response::new(response))
    }

    async fn create_group(
        &self,
        request: Request<CreateGroupReq>,
    ) -> Result<Response<CommonResp>, Status> {
        todo!()
    }

    async fn destroy_group(
        &self,
        request: Request<DestroyGroupReq>,
    ) -> Result<Response<CommonResp>, Status> {
        todo!()
    }

    async fn get_group_info(
        &self,
        request: Request<GetGroupInfoReq>,
    ) -> Result<Response<GetGroupInfoRep>, Status> {
        todo!()
    }

    async fn update_group_info(
        &self,
        request: Request<UpdateGroupInfoReq>,
    ) -> Result<Response<CommonResp>, Status> {
        todo!()
    }

    async fn add_member(
        &self,
        request: Request<AddMemberReq>,
    ) -> Result<Response<CommonResp>, Status> {
        todo!()
    }

    async fn remove_member(
        &self,
        request: Request<RemoveMemberReq>,
    ) -> Result<Response<CommonResp>, Status> {
        todo!()
    }

    async fn exit_group(
        &self,
        request: Request<ExitGroupReq>,
    ) -> Result<Response<CommonResp>, Status> {
        todo!()
    }

    async fn change_member_role(
        &self,
        request: Request<ChangeMemberRoleReq>,
    ) -> Result<Response<CommonResp>, Status> {
        todo!()
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
