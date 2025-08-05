use crate::protocol::rpc::arb_models::{NodeInfo, NodeType, QueryNodeReq};
use crate::protocol::rpc::arb_server::arb_server_rpc_service_client;
use anyhow::Result;
use common::config::AppConfig;
use std::sync::Arc;

#[derive(Debug)]
pub struct ArbClient {
    pub client: arb_server_rpc_service_client::ArbServerRpcServiceClient<tonic::transport::Channel>,
    pub node_list: Vec<NodeInfo>,
}
impl ArbClient {
    pub async fn new() -> Self {
        let server_host = AppConfig::get().get_shard().server_host.unwrap();
        let channel = arb_server_rpc_service_client::ArbServerRpcServiceClient::connect(server_host).await.unwrap();
        Self {
            client: channel,
            node_list: vec![],
        }
    }
    pub async fn init() {
        let client = self::ArbClient::new().await;
        let _ = INSTANCE.set(Arc::new(client));
    }
    pub fn get() -> Arc<Self> {
        INSTANCE.get().expect("INSTANCE is not initialized").clone()
    }
    pub async fn get_node_list(&mut self) -> Result<Vec<NodeInfo>> {
        let request = tonic::Request::new(QueryNodeReq {
            node_type: NodeType::MsgGateway as i32,
        });
        let response = self.client.list_all_nodes(request).await?;
        self.node_list = response.into_inner().nodes;
        Ok(self.node_list.clone())
    }
}

static INSTANCE: once_cell::sync::OnceCell<Arc<ArbClient>> = once_cell::sync::OnceCell::new();
