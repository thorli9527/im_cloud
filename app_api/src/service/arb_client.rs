use biz_service::protocol::rpc::arb_server::arb_server_rpc_service_client;
use common::config::AppConfig;

pub struct ArbClient {
    pub client: arb_server_rpc_service_client::ArbServerRpcServiceClient<tonic::transport::Channel>,
}
impl ArbClient {
    pub async fn new() -> Self {
        let server_host = AppConfig::get().get_shard().server_host.unwrap();
        let channel = arb_server_rpc_service_client::ArbServerRpcServiceClient::connect(server_host).await.unwrap();
        Self {
            client: channel,
        }
    }
}
