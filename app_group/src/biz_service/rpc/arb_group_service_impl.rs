use tonic::{Request, Response, Status};
use crate::protocol::rpc_arb_group;
use crate::protocol::rpc_arb_group::UpdateVersionReq;
use crate::protocol::rpc_group_models::{AddMemberReq, ChangeMemberAliasReq, ChangeMemberRoleReq, CommonResp, CreateGroupReq, DestroyGroupReq, ExitGroupReq, GetGroupInfoRep, GetGroupInfoReq, GetMembersRep, GetMembersReq, GetOnlineMembersReq, GetOnlineMembersResp, MuteMemberReq, OfflineReq, OnlineReq, RemoveMemberReq, TransferGroupOwnershipReq, UpdateGroupInfoReq};

/// arb 组 客户端接口
struct ArbGroupServiceImpl {
    
}
#[tonic::async_trait]
impl rpc_arb_group::arb_group_service_server::ArbGroupService for ArbGroupServiceImpl {
    async fn change_preparing(&self, request: Request<()>) -> Result<Response<crate::protocol::rpc_arb_models::CommonResp>, Status> {
        todo!()
    }

    async fn update_version(&self, request: Request<UpdateVersionReq>) -> Result<Response<crate::protocol::rpc_arb_models::CommonResp>, Status> {
        todo!()
    }
}