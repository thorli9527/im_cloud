use tonic::{Request, Response, Status};
use biz_core::protocol::common::{CommonResp, IdReq};
use biz_core::protocol::msg::friend_msg_server::{AcceptFriendReqMsg, AddFriendReqMsg, ChangeFriendReqMsg, DeleteFriendReqMsg, FriendListRespMsg, SendMessageRespMsg};
use biz_core::protocol::msg::friend_msg_server::friend_rpc_service_server::FriendRpcService;

#[derive(Debug)]
pub struct FriendRpcServiceImpl{

}
#[tonic::async_trait]
impl FriendRpcService for FriendRpcServiceImpl {
    async fn add_friend_req(&self, request: Request<AddFriendReqMsg>) -> Result<Response<CommonResp>, Status> {
        todo!()
    }

    async fn accept_friend_req(&self, request: Request<AcceptFriendReqMsg>) -> Result<Response<CommonResp>, Status> {
        todo!()
    }

    async fn delete_friend(&self, request: Request<DeleteFriendReqMsg>) -> Result<Response<CommonResp>, Status> {
        todo!()
    }

    async fn get_friend_list(&self, request: Request<IdReq>) -> Result<Response<FriendListRespMsg>, Status> {
        todo!()
    }

    async fn send_message(&self, request: Request<SendMessageRespMsg>) -> Result<Response<CommonResp>, Status> {
        todo!()
    }

    async fn change_friend(&self, request: Request<ChangeFriendReqMsg>) -> Result<Response<CommonResp>, Status> {
        todo!()
    }
}