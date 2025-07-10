use tonic::{Request, Response, Status};
use crate::protocol::arbitration::arbiter_service_server::ArbiterService;
use crate::protocol::arbitration::{GetNodeInfoRequest, GetNodeInfoResponse, GetShardRequest, GetShardResponse, GracefulLeaveRequest, GracefulLeaveResponse, IsLocalShardRequest, IsLocalShardResponse, ListNodesRequest, ListNodesResponse, ListShardsRequest, ListShardsResponse, NodeHeartbeatRequest, NodeHeartbeatResponse, RegisterNodeRequest, RegisterNodeResponse, RemoveNodeRequest, RemoveNodeResponse, UpdateOwnerRequest, UpdateOwnerResponse, UpdateShardStateRequest, UpdateShardStateResponse};

struct ArbiterServiceImpl{

}
#[tonic::async_trait]
impl ArbiterService for ArbiterServiceImpl{
    async fn get_shard(&self, request: Request<GetShardRequest>) -> Result<Response<GetShardResponse>, Status> {
        todo!()
    }

    async fn update_owner(&self, request: Request<UpdateOwnerRequest>) -> Result<Response<UpdateOwnerResponse>, Status> {
        todo!()
    }

    async fn update_shard_state(&self, request: Request<UpdateShardStateRequest>) -> Result<Response<UpdateShardStateResponse>, Status> {
        todo!()
    }

    async fn list_shards(&self, request: Request<ListShardsRequest>) -> Result<Response<ListShardsResponse>, Status> {
        todo!()
    }

    async fn node_heartbeat(&self, request: Request<NodeHeartbeatRequest>) -> Result<Response<NodeHeartbeatResponse>, Status> {
        todo!()
    }

    async fn is_local_shard(&self, request: Request<IsLocalShardRequest>) -> Result<Response<IsLocalShardResponse>, Status> {
        todo!()
    }

    async fn register_node(&self, request: Request<RegisterNodeRequest>) -> Result<Response<RegisterNodeResponse>, Status> {
        todo!()
    }

    async fn remove_node(&self, request: Request<RemoveNodeRequest>) -> Result<Response<RemoveNodeResponse>, Status> {
        todo!()
    }

    async fn list_nodes(&self, request: Request<ListNodesRequest>) -> Result<Response<ListNodesResponse>, Status> {
        todo!()
    }

    async fn get_node_info(&self, request: Request<GetNodeInfoRequest>) -> Result<Response<GetNodeInfoResponse>, Status> {
        todo!()
    }

    async fn graceful_leave(&self, request: Request<GracefulLeaveRequest>) -> Result<Response<GracefulLeaveResponse>, Status> {
        todo!()
    }
}