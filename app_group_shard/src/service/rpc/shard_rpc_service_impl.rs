use crate::service::shard_manager::{ShardManager, ShardManagerOpt};
use std::sync::Arc;
use tonic::{Request, Response, Status};
use biz_service::protocol::arb::shard_service::{AddMemberReq, ChangeRoleReq, GetGroupsResp, GetMemberCountReq, GetMemberPageReq, MemberCountResp, MemberListResp, OnlineReq, RemoveMemberReq, UserIdListResp};
use biz_service::protocol::arb::shard_service::shard_rpc_service_server::ShardRpcService;
use biz_service::protocol::common::IdReq;

pub struct ShardRpcServiceImpl {
    pub shard_manager: Arc<ShardManager>,
}

impl ShardRpcServiceImpl {
    pub fn new() -> Self {
        Self {
            shard_manager: ShardManager::get(),
        }
    }
}
#[tonic::async_trait]
impl ShardRpcService for ShardRpcServiceImpl {
    async fn create(&self, request: Request<IdReq>) -> Result<Response<()>, Status> {
        let req = request.into_inner();
        self.shard_manager.create(&req.ref_id).await.unwrap();
        Ok(Response::new(()))
    }

    async fn dismiss(&self, request: Request<IdReq>) -> Result<Response<()>, Status> {
        let req = request.into_inner();
        self.shard_manager.dismiss(&req.ref_id);
        Ok(Response::new(()))
    }

    async fn add_member(&self, request: Request<AddMemberReq>) -> Result<Response<()>, Status> {
        let req = request.into_inner();
        self.shard_manager.add_member(&req.group_id, &req.user_id, req.role()).unwrap();
        Ok(Response::new(()))
    }

    async fn remove_member(
        &self,
        request: Request<RemoveMemberReq>,
    ) -> Result<Response<()>, Status> {
        let req = request.into_inner();
        self.shard_manager.remove_member(&req.group_id, &req.user_id).unwrap();
        Ok(Response::new(()))
    }

    async fn get_member(
        &self,
        request: Request<IdReq>,
    ) -> Result<Response<MemberListResp>, Status> {
        let req = request.into_inner();
        let member_list = self.shard_manager.get_member(&req.ref_id).unwrap();
        Ok(Response::new(MemberListResp {
            members: member_list.into_iter().map(|m| m.into()).collect(),
        }))
    }

    async fn get_member_page(
        &self,
        request: Request<GetMemberPageReq>,
    ) -> Result<Response<MemberListResp>, Status> {
        let req = request.into_inner();
        let member_list = self
            .shard_manager
            .get_member_page(&req.group_id, req.offset as usize, req.limit as usize)
            .unwrap();
        Ok(Response::new(MemberListResp {
            members: member_list.unwrap(),
        }))
    }

    async fn get_member_count(
        &self,
        request: Request<GetMemberCountReq>,
    ) -> Result<Response<MemberCountResp>, Status> {
        let req = request.into_inner();
        let member_count = self.shard_manager.get_member_count(&req.group_id).unwrap();
        Ok(Response::new(MemberCountResp {
            count: member_count as u32,
        }))
    }

    async fn online(&self, request: Request<OnlineReq>) -> Result<Response<()>, Status> {
        let req = request.into_inner();
        self.shard_manager.online(&req.user_id, &req.group_id).unwrap();
        Ok(Response::new(()))
    }

    async fn get_online_member(
        &self,
        request: Request<IdReq>,
    ) -> Result<Response<UserIdListResp>, Status> {
        let req = request.into_inner();
        Ok(Response::new(UserIdListResp {
            user_ids: self.shard_manager.get_on_line_member(&req.ref_id),
        }))
    }

    async fn change_role(&self, request: Request<ChangeRoleReq>) -> Result<Response<()>, Status> {
        let req = request.into_inner();
        self.shard_manager.change_role(&req.group_id, &req.user_id, req.role()).unwrap();
        Ok(Response::new(()))
    }

    async fn get_admin_member(
        &self,
        request: Request<IdReq>,
    ) -> Result<Response<UserIdListResp>, Status> {
        let req = request.into_inner();
        let user_ids = self.shard_manager.get_admin_member(&req.ref_id).await;
        let option = user_ids.expect("get_admin_member.error");
        Ok(Response::new(UserIdListResp {
            user_ids: option.unwrap(),
        }))
    }

    async fn get_user_groups(
        &self,
        request: Request<IdReq>,
    ) -> Result<Response<GetGroupsResp>, Status> {
        let req = request.into_inner();
        let groups = self.shard_manager.get_user_groups(&req.ref_id).unwrap();

        Ok(Response::new(GetGroupsResp {
            group_ids: groups,
        }))
    }
}
