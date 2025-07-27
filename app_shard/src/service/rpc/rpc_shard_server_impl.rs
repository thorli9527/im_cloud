use crate::service::shard_manager::{ShardManager, ShardManagerOpt};
use async_trait::async_trait;
use biz_service::protocol::rpc::rpc_shard_server::shard_rpc_service_server::ShardRpcService;
use biz_service::protocol::rpc::rpc_shard_server::{
    AddMemberReq, ChangeRoleReq, GetMemberCountReq, GetMemberPageReq, IdReq, MemberCountResp, MemberListResp, OnlineReq, RemoveMemberReq,
    UserIdListResp,
};
use once_cell::sync::OnceCell;
use std::sync::Arc;
use tonic::{Code, Request, Response, Status};

#[derive(Debug)]
pub struct ShardRPCServiceImpl {}

impl ShardRPCServiceImpl {
    pub fn new() -> Self {
        Self {}
    }
    pub async fn init() {
        let instance = Self::new();
        INSTANCE.set(Arc::new(instance)).expect("TODO: panic message");
    }

    /// 获取单例
    pub fn get() -> Arc<Self> {
        INSTANCE.get().expect("INSTANCE is not initialized").clone()
    }
}
static INSTANCE: OnceCell<Arc<ShardRPCServiceImpl>> = OnceCell::new();
#[async_trait]
impl ShardRpcService for ShardRPCServiceImpl {
    async fn create(&self, request: Request<IdReq>) -> Result<Response<()>, Status> {
        let req = request.into_inner();
        let shard_manager = ShardManager::get();
        let result = shard_manager.create(&req.ref_id).await;
        return match result {
            Ok(_) => Ok(Response::new(())),
            Err(e) => Err(Status::new(Code::Internal, e.to_string())),
        };
    }

    async fn dismiss(&self, request: Request<IdReq>) -> Result<Response<()>, Status> {
        let req = request.into_inner();
        let shard_manager = ShardManager::get();
        shard_manager.dismiss(&req.ref_id);
        Ok(Response::new(()))
    }

    async fn add_member(&self, request: Request<AddMemberReq>) -> Result<Response<()>, Status> {
        let req = request.into_inner();
        let shard_manager = ShardManager::get();
        let result = shard_manager.add_member(&req.group_id, &req.user_id, req.role());
        return match result {
            Ok(_) => Ok(Response::new(())),
            Err(e) => Err(Status::new(Code::Internal, e.to_string())),
        };
    }

    async fn remove_member(&self, request: Request<RemoveMemberReq>) -> Result<Response<()>, Status> {
        let req = request.into_inner();
        let shard_manager = ShardManager::get();
        let result = shard_manager.remove_member(&req.group_id, &req.user_id);
        return match result {
            Ok(_) => Ok(Response::new(())),
            Err(e) => Err(Status::new(Code::Internal, e.to_string())),
        };
    }

    async fn get_member(&self, request: Request<IdReq>) -> Result<Response<MemberListResp>, Status> {
        let req = request.into_inner();
        let shard_manager = ShardManager::get();
        let result = shard_manager.get_member(&req.ref_id);
        return match result {
            Ok(member_list) => Ok(Response::new(MemberListResp {
                members: member_list,
            })),
            Err(e) => Err(Status::new(Code::Internal, e.to_string())),
        };
    }

    async fn get_member_page(&self, request: Request<GetMemberPageReq>) -> Result<Response<MemberListResp>, Status> {
        let req = request.into_inner();
        let shard_manager = ShardManager::get();
        let result = shard_manager.get_member_page(&req.group_id, req.offset as usize, req.limit as usize);
        return match result {
            Ok(Some(member_list)) => Ok(Response::new(MemberListResp {
                members: member_list,
            })),
            Ok(None) => Ok(Response::new(MemberListResp {
                members: vec![],
            })),
            Err(e) => Err(Status::new(Code::Internal, e.to_string())),
        };
    }

    async fn get_member_count(&self, request: Request<GetMemberCountReq>) -> Result<Response<MemberCountResp>, Status> {
        let req = request.into_inner();
        let shard_manager = ShardManager::get();
        let result = shard_manager.get_member_count(&req.group_id);
        return match result {
            Ok(count) => Ok(Response::new(MemberCountResp {
                count: count as u32,
            })),
            Err(e) => Err(Status::new(Code::Internal, e.to_string())),
        };
    }

    async fn online(&self, request: Request<OnlineReq>) -> Result<Response<()>, Status> {
        let req = request.into_inner();
        let shard_manager = ShardManager::get();
        let result = shard_manager.online(&req.group_id, &req.user_id);
        return match result {
            Ok(_) => Ok(Response::new(())),
            Err(e) => Err(Status::new(Code::Internal, e.to_string())),
        };
    }

    async fn get_online_member(&self, request: Request<IdReq>) -> Result<Response<UserIdListResp>, Status> {
        let req = request.into_inner();
        let shard_manager = ShardManager::get();
        let result = shard_manager.get_on_line_member(&req.ref_id);
        Ok(Response::new(UserIdListResp {
            user_ids: result,
        }))
    }

    async fn change_role(&self, request: Request<ChangeRoleReq>) -> Result<Response<()>, Status> {
        let req = request.into_inner();
        let shard_manager = ShardManager::get();
        let result = shard_manager.change_role(&req.group_id, &req.user_id, req.role());
        return match result {
            Ok(_) => Ok(Response::new(())),
            Err(e) => Err(Status::new(Code::Internal, e.to_string())),
        };
    }

    async fn get_admin_member(&self, request: Request<IdReq>) -> Result<Response<UserIdListResp>, Status> {
        let req = request.into_inner();
        let shard_manager = ShardManager::get();
        let result = shard_manager.get_admin_member(&req.ref_id).await;
        return match result {
            Ok(list) => Ok(Response::new(UserIdListResp {
                user_ids: list.unwrap(),
            })),
            Err(e) => Err(Status::new(Code::Internal, e.to_string())),
        };
    }
}
