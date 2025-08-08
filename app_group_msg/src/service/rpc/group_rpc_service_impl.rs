use tonic::{Request, Response, Status};
use biz_core::protocol::common::CommonResp;
use biz_core::protocol::msg::group_msg_server::{CreateGroupReq, DismissGroupReq, GroupMessageReq, InviteMemberReq, JoinGroupReq, KickMemberReq, QuitGroupReq, UpdateGroupProfileReq};
use biz_core::protocol::msg::group_msg_server::group_rpc_service_server::GroupRpcService;

#[derive(Debug)]
pub struct GroupRpcServiceImpl{
    
}
#[tonic::async_trait]
impl GroupRpcService for GroupRpcServiceImpl{
    async fn create_group(&self, request: Request<CreateGroupReq>) -> Result<Response<CommonResp>, Status> {
        todo!()
    }

    async fn update_group_profile(&self, request: Request<UpdateGroupProfileReq>) -> Result<Response<CommonResp>, Status> {
        todo!()
    }

    async fn dismiss_group(&self, request: Request<DismissGroupReq>) -> Result<Response<CommonResp>, Status> {
        todo!()
    }

    async fn join_group(&self, request: Request<JoinGroupReq>) -> Result<Response<CommonResp>, Status> {
        todo!()
    }

    async fn quit_group(&self, request: Request<QuitGroupReq>) -> Result<Response<CommonResp>, Status> {
        todo!()
    }

    async fn kick_member(&self, request: Request<KickMemberReq>) -> Result<Response<CommonResp>, Status> {
        todo!()
    }

    async fn invite_member(&self, request: Request<InviteMemberReq>) -> Result<Response<CommonResp>, Status> {
        todo!()
    }

    async fn send_group_message(&self, request: Request<GroupMessageReq>) -> Result<Response<CommonResp>, Status> {
        todo!()
    }
}