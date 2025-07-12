use tonic::{Request, Response, Status};
use crate::protocol::rpc_group_models::{AddMemberReq, ChangeMemberAliasReq, ChangeMemberRoleReq, CommonResp, CreateGroupReq, DestroyGroupReq, ExitGroupReq, GetGroupInfoRep, GetGroupInfoReq, GetMembersRep, GetMembersReq, GetOnlineMembersReq, GetOnlineMembersResp, MuteMemberReq, OfflineReq, OnlineReq, RemoveMemberReq, TransferGroupOwnershipReq, UpdateGroupInfoReq};
use crate::protocol::rpc_group_server::group_rpc_service_server::GroupRpcService;
/// 群聊服务端rpc服务
struct GroupRpcServiceImpl {
    // 这里可以添加一些共享状态或依赖注入
    
}
#[tonic::async_trait]
impl GroupRpcService for GroupRpcServiceImpl{
    async fn online(&self, request: Request<OnlineReq>) -> Result<Response<CommonResp>, Status> {
        todo!()
    }

    async fn offline(&self, request: Request<OfflineReq>) -> Result<Response<CommonResp>, Status> {
        todo!()
    }

    async fn get_online_members(&self, request: Request<GetOnlineMembersReq>) -> Result<Response<GetOnlineMembersResp>, Status> {
        todo!()
    }

    async fn get_members(&self, request: Request<GetMembersReq>) -> Result<Response<GetMembersRep>, Status> {
        todo!()
    }

    async fn create_group(&self, request: Request<CreateGroupReq>) -> Result<Response<CommonResp>, Status> {
        todo!()
    }

    async fn destroy_group(&self, request: Request<DestroyGroupReq>) -> Result<Response<CommonResp>, Status> {
        todo!()
    }

    async fn get_group_info(&self, request: Request<GetGroupInfoReq>) -> Result<Response<GetGroupInfoRep>, Status> {
        todo!()
    }


    async fn update_group_info(&self, request: Request<UpdateGroupInfoReq>) -> Result<Response<CommonResp>, Status> {
        todo!()
    }

    async fn add_member(&self, request: Request<AddMemberReq>) -> Result<Response<CommonResp>, Status> {
        todo!()
    }

    async fn remove_member(&self, request: Request<RemoveMemberReq>) -> Result<Response<CommonResp>, Status> {
        todo!()
    }

    async fn exit_group(&self, request: Request<ExitGroupReq>) -> Result<Response<CommonResp>, Status> {
        todo!()
    }

    async fn change_member_role(&self, request: Request<ChangeMemberRoleReq>) -> Result<Response<CommonResp>, Status> {
        todo!()
    }

    async fn mute_member(&self, request: Request<MuteMemberReq>) -> Result<Response<CommonResp>, Status> {
        todo!()
    }

    async fn change_member_alias(&self, request: Request<ChangeMemberAliasReq>) -> Result<Response<CommonResp>, Status> {
        todo!()
    }

    async fn transfer_ownership(&self, request: Request<TransferGroupOwnershipReq>) -> Result<Response<CommonResp>, Status> {
        todo!()
    }
}