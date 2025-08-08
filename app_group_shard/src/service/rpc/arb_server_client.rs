use biz_core::protocol::arb::arb_server::arb_server_rpc_service_client;
use common::config::AppConfig;

#[derive(Debug)]
pub struct ArbServerClient {
    pub client: arb_server_rpc_service_client::ArbServerRpcServiceClient<tonic::transport::Channel>,
}
impl ArbServerClient {
    pub async fn new() -> Self {
        let server_host = AppConfig::get().get_shard().server_addr.unwrap();
        let channel = arb_server_rpc_service_client::ArbServerRpcServiceClient::connect(server_host).await.unwrap();
        Self {
            client: channel,
        }
    }
}
