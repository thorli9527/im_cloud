mod biz_service;
mod protocol;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::biz_service::grpc::arb_service_impl::{ArbiterServiceImpl};
use crate::protocol::arbitration::arbiter_service_server::ArbiterServiceServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "127.0.0.1:50051".parse().unwrap();
    let svc = ArbiterServiceImpl {
        shard_nodes: Arc::new(Default::default()),
    };

    tonic::transport::Server::builder()
        .add_service(ArbiterServiceServer::new(svc))
        .serve(addr)
        .await?;

    Ok(())
}
