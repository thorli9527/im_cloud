use anyhow::{Result, anyhow};
use dashmap::DashMap;
use once_cell::sync::OnceCell;
use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::transport::Channel;

use crate::protocol::rpc_arb_models::{NodeType, QueryNodeReq};
use crate::protocol::rpc_arb_server::arb_server_rpc_service_client::ArbServerRpcServiceClient;

use crate::protocol::common::ByteMessageType;
use common::config::AppConfig;
use common::util::common_utils::{build_md5, hash_index};
use prost::Message;
use rdkafka::producer::FutureProducer;
use biz_service::biz_service::kafka_group_service::KafkaGroupService;
use biz_service::protocol::msg::group_models::GroupNodeMsgType;

static GROUP_SERVER_KAFKA: OnceCell<DashMap<i32, String>> = OnceCell::new();
static GROUP_SERVER_KAFKA_CLIENT: OnceCell<DashMap<i32, Arc<KafkaGroupService>>> = OnceCell::new();
static ARB_SERVER_CLIENT: OnceCell<Mutex<ArbServerRpcServiceClient<Channel>>> = OnceCell::new();

pub struct ArbServerClient {}

impl ArbServerClient {
    pub fn new() -> Self {
        Self {}
    }

    /// 获取指定 group 的 Kafka producer，根据一致性哈希映射
    pub fn get_group_kafka(group_id: &str) -> Result<Arc<FutureProducer>> {
        let client_map = GROUP_SERVER_KAFKA_CLIENT
            .get()
            .ok_or_else(|| anyhow!("GROUP_SERVER_KAFKA_CLIENT not initialized"))?;

        let total = client_map.len() as i32;
        if total == 0 {
            return Err(anyhow!("No group Kafka clients registered"));
        }

        let index = hash_index(group_id, total);
        let client = client_map.get(&index).ok_or_else(|| {
            anyhow!(
                "Kafka client not found for group_id: {}, index: {}",
                group_id,
                index
            )
        })?;

        Ok(client.producer.clone())
    }

    /// 初始化 gRPC 客户端（Arb + Group 节点）
    pub async fn init_grpc_client() -> Result<()> {
        let client = ArbServerClient::new();
        client.init_arb_client().await?;
        client.refresh_group_kafka_list().await?;
        Ok(())
    }

    /// 初始化仲裁服务客户端（只执行一次）
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

    /// 根据 group_id 获取 Kafka producer 并发送 protobuf 消息（带类型字节）
    pub async fn send_proto<M: Message>(
        &self,
        group_id: &str,
        msg_type: &GroupNodeMsgType,
        message: &M,
        message_id: &str,
        topic: &str,
    ) -> Result<()> {
        let client_map = GROUP_SERVER_KAFKA_CLIENT
            .get()
            .ok_or_else(|| anyhow!("GROUP_SERVER_KAFKA_CLIENT not initialized"))?;

        let total = client_map.len() as i32;
        if total == 0 {
            return Err(anyhow!("No group Kafka clients registered"));
        }

        let index = hash_index(group_id, total);
        let service = client_map.get(&index).ok_or_else(|| {
            anyhow!(
                "KafkaGroupService not found for group_id: {}, index: {}",
                group_id,
                index
            )
        })?;

        // 调用 KafkaGroupService 的 send_proto 方法
        service
            .send_proto(msg_type, message, message_id)
            .await
            .map_err(|e| anyhow!("Kafka send_proto failed: {e}"))
    }

    /// 获取 Arb 服务客户端（自动 clone）
    pub async fn get_arb_server_client(&self) -> Result<ArbServerRpcServiceClient<Channel>> {
        let lock = ARB_SERVER_CLIENT
            .get()
            .ok_or_else(|| anyhow!("ArbServerRpcServiceClient not initialized"))?;
        let client = lock.lock().await;
        Ok(client.clone())
    }

    /// 刷新 group 节点列表（可多次调用）
    pub async fn refresh_group_kafka_list(&self) -> Result<()> {
        let mut client = self.get_arb_server_client().await?;

        let node_list = client
            .list_all_nodes(QueryNodeReq {
                node_type: NodeType::GroupNode as i32,
            })
            .await
            .map_err(|e| anyhow!("list_all_nodes failed: {e}"))?
            .into_inner()
            .nodes;

        let total = node_list.len() as i32;
        if total == 0 {
            return Err(anyhow!("No group nodes found from arb server"));
        }

        // 更新 IP 映射表
        let ip_map = GROUP_SERVER_KAFKA.get_or_init(DashMap::new);
        ip_map.clear();

        // 更新 Kafka 客户端表
        let client_map = GROUP_SERVER_KAFKA_CLIENT.get_or_init(DashMap::new);
        client_map.clear();

        for node in &node_list {
            let parts: Vec<&str> = node.node_addr.split(':').collect();
            let ip = parts
                .get(0)
                .ok_or_else(|| anyhow!("Invalid node address: {}", node.node_addr))?;
            let index = hash_index(ip, total);

            ip_map.insert(index, ip.to_string());

            // 统一的 Kafka 用户与密码构建方式
            let broker = format!("{}:9092", ip);
            let user_name = "group_server";
            let password = build_md5(&broker);

            let kafka_client = KafkaGroupService::new(&broker, user_name, &password)
                .map_err(|e| anyhow!("Failed to create KafkaGroupService for {}: {}", broker, e))?;

            client_map.insert(index, Arc::new(kafka_client));
        }

        Ok(())
    }
}
