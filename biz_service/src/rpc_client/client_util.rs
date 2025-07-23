use anyhow::{Result, anyhow};
use dashmap::DashMap;
use once_cell::sync::OnceCell;
use prost::Message;
use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::transport::Channel;

use crate::biz_service::kafka_group_service::KafkaGroupService;
use crate::protocol::arb::rpc_arb_models::{NodeType, QueryNodeReq};
use crate::protocol::arb::rpc_arb_server::arb_server_rpc_service_client::ArbServerRpcServiceClient;
use crate::protocol::msg::group::GroupNodeMsgType;
use common::config::AppConfig;
use common::util::common_utils::hash_index;

static INSTANCE_SERVICE: OnceCell<Arc<ArbServerClient>> = OnceCell::new();
static GROUP_SERVER_KAFKA_CLIENT: OnceCell<DashMap<i32, Arc<KafkaGroupService>>> = OnceCell::new();
static ARB_SERVER_CLIENT: OnceCell<Mutex<ArbServerRpcServiceClient<Channel>>> = OnceCell::new();

#[derive(Debug, Clone)]
pub struct ArbServerClient {}
impl ArbServerClient {
    pub fn new() -> Self {
        Self {}
    }

    /// 获取 ArbServerClient 单例
    pub async fn get() -> Result<Arc<Self>> {
        if let Some(instance) = INSTANCE_SERVICE.get() {
            return Ok(instance.clone());
        }

        // 还未初始化则进行初始化
        Self::init().await?;

        INSTANCE_SERVICE
            .get()
            .cloned()
            .ok_or_else(|| anyhow!("ArbServerClient instance not initialized after init"))
    }

    /// 初始化 ArbServerClient 实例
    pub async fn init() -> Result<()> {
        if INSTANCE_SERVICE.get().is_some() {
            return Ok(()); // 已初始化则跳过
        }

        let client = ArbServerClient::new();
        let arc_client = Arc::new(client.clone());

        // 注册全局实例
        INSTANCE_SERVICE
            .set(arc_client.clone())
            .map_err(|_| anyhow!("ArbServerClient instance already initialized"))?;

        // 初始化 gRPC 和 Kafka 客户端
        client.init_arb_client().await?;
        client.refresh_group_kafka_list().await?;

        Ok(())
    }

    /// 获取 Arb 服务 gRPC 客户端
    pub async fn get_arb_server_client(&self) -> Result<ArbServerRpcServiceClient<Channel>> {
        let lock = ARB_SERVER_CLIENT
            .get()
            .ok_or_else(|| anyhow!("ArbServerRpcServiceClient not initialized"))?;
        let client = lock.lock().await;
        Ok(client.clone())
    }

    async fn init_arb_client(&self) -> Result<()> {
        if ARB_SERVER_CLIENT.get().is_some() {
            return Ok(());
        }

        let app_config = AppConfig::get();
        let addr = format!("http://{}", app_config.get_server().host);
        let client = ArbServerRpcServiceClient::connect(addr).await?;
        ARB_SERVER_CLIENT
            .set(Mutex::new(client))
            .map_err(|_| anyhow!("ArbServerRpcServiceClient already initialized"))?;
        Ok(())
    }

    pub async fn refresh_group_kafka_list(&self) -> Result<()> {
        let mut client = self.get_arb_server_client().await?;

        let node_list = client
            .list_all_nodes(QueryNodeReq { node_type: NodeType::GroupNode as i32 })
            .await
            .map_err(|e| anyhow!("list_all_nodes failed: {e}"))?
            .into_inner()
            .nodes;

        let total = node_list.len() as i32;
        if total == 0 {
            return Err(anyhow!("No group nodes found from arb server"));
        }

        let client_map = GROUP_SERVER_KAFKA_CLIENT.get_or_init(DashMap::new);
        client_map.clear();

        for node in &node_list {
            let parts: Vec<&str> = node.node_addr.split(':').collect();
            let ip =
                parts.get(0).ok_or_else(|| anyhow!("Invalid node address: {}", node.node_addr))?;
            let index = hash_index(ip, total);
            let broker = format!("{}:9092", ip);
            let kafka_client = KafkaGroupService::new(&broker).await?;
            client_map.insert(index, Arc::new(kafka_client));
        }

        Ok(())
    }

    /// 获取 Kafka producer
    async fn get_group_kafka(&self, group_id: &str) -> Result<Arc<KafkaGroupService>> {
        let client_map = GROUP_SERVER_KAFKA_CLIENT
            .get()
            .ok_or_else(|| anyhow!("GROUP_SERVER_KAFKA_CLIENT not initialized"))?;

        let total = client_map.len() as i32;
        if total == 0 {
            return Err(anyhow!("No group Kafka clients registered"));
        }

        let index = hash_index(group_id, total);
        let client = client_map.get(&index).ok_or_else(|| {
            anyhow!("Kafka client not found for group_id: {}, index: {}", group_id, index)
        })?;
        Ok(client.clone())
    }

    pub async fn send_msg<M: Message>(
        &self,
        group_id: &str,
        msg_type: &GroupNodeMsgType,
        message: &M,
        message_id: &str,
    ) -> Result<()> {
        let kafka_client = self.get_group_kafka(group_id).await?;
        kafka_client.send_proto(msg_type, message, message_id).await?;
        Ok(())
    }
}
