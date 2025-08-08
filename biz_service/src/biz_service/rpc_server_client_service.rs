use common::config::AppConfig;
use once_cell::sync::OnceCell;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::protocol::arb::arb_server::arb_server_rpc_service_client::ArbServerRpcServiceClient;

#[derive(Debug)]
pub struct ArbServerRpcServiceClientService {
    pub client: Arc<Mutex<ArbServerRpcServiceClient<tonic::transport::Channel>>>,
}
impl ArbServerRpcServiceClientService {
    async fn new() -> Self {
        let string = AppConfig::get().get_shard().clone().server_addr.unwrap();
        Self {
            client: Arc::new(Mutex::new(ArbServerRpcServiceClient::connect(format!("http://{}", string)).await.unwrap())),
        }
    }
    pub fn get() -> Arc<Self> {
        INSTANCE.get().unwrap().clone()
    }
    pub async fn init() -> anyhow::Result<()> {
        INSTANCE.set(Arc::new(Self::new().await)).expect("Failed to set instance");
        return Ok(());
    }
}

static INSTANCE: OnceCell<Arc<ArbServerRpcServiceClientService>> = OnceCell::new();
