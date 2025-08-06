// 在 app_socket 中添加节点注册与心跳逻辑到 app_arb 服务

use crate::kafka;
use biz_service::biz_service::kafka_group_service::KafkaGroupService;
use biz_service::protocol::rpc::arb_models::{BaseRequest, NodeInfo, NodeType, QueryNodeReq, RegRequest};
use biz_service::protocol::rpc::arb_server::arb_server_rpc_service_client::ArbServerRpcServiceClient;
use common::config::{AppConfig, ShardConfig};
use common::util::common_utils::{build_md5, hash_index};
use config::Config;
use once_cell::sync::OnceCell;
use std::clone;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tonic::transport::Channel;

#[derive(Debug, Clone)]
pub struct ArbClient {
    pub arb_client: ArbServerRpcServiceClient<Channel>,

    ///当前节点地址
    pub node_addr: String,
    /// kafka地址
    pub kafka_addr: String,
    /// socket服务地址
    pub socket_addr: String,
    /// shard kafka服务
    pub shard_kafka_list: HashMap<i32, KafkaGroupService>,
}

impl ArbClient {
    /// 创建 ArbClient 并注册自身地址
    pub async fn new(shard_config: &ShardConfig, kafka_addr: &str, socket_addr: &str) -> anyhow::Result<Self> {
        let node_addr = shard_config.server_addr.clone().ok_or_else(|| anyhow::anyhow!("Missing server_host in ShardConfig"))?;
        let client = ArbServerRpcServiceClient::connect(format!("http://{}", node_addr)).await?;
        Ok(Self {
            arb_client: client,
            shard_kafka_list: HashMap::new(),
            node_addr,
            socket_addr: socket_addr.to_string(),
            kafka_addr: kafka_addr.to_string(),
        })
    }
    pub fn get() -> Arc<RwLock<ArbClient>> {
        INSTANCE.get().expect("ArbClient is not initialized").clone()
    }
    /// 注册本节点 + 初始化 shard_clients
    pub async fn init() -> anyhow::Result<()> {
        let shard_config = AppConfig::get().shard.clone().unwrap();
        let kafka_addr = AppConfig::get().kafka.clone().unwrap();
        let socket_addr = AppConfig::get().socket.clone().unwrap();
        let mut arb_client = ArbClient::new(&shard_config, &kafka_addr.brokers, &socket_addr.node_addr).await?;

        arb_client.register().await?;

        // 初始化 shard clients 列表
        arb_client.init_shard_kafka_list().await?;

        INSTANCE.set(Arc::new(RwLock::new(arb_client))).expect("ArbClient init failed");
        Ok(())
    }
    pub fn get_shard_kafka(&self, group_id: &str, shard_id: u32) -> anyhow::Result<KafkaGroupService> {
        let shard_group_index = hash_index(group_id, self.shard_kafka_list.len() as i32);
        // for (node_addr, client) in &self.shard_clients {
        //     let shard_node_index = hash_index(&node_addr, self.shard_clients.len() as i32);
        //     if shard_node_index == shard_group_index {
        //         return Ok(client.clone());
        //     }
        // }
        Err(anyhow::anyhow!("no shard node"))
    }
    pub async fn register(&mut self) -> anyhow::Result<NodeInfo> {
        let req = RegRequest {
            node_addr: self.node_addr.clone(),
            node_type: NodeType::SocketNode as i32,
            kafka_addr: Some(self.socket_addr.clone()),
        };
        let resp = self.arb_client.register_node(req).await?.into_inner();
        Ok(resp)
    }

    /// 根据仲裁服务返回的最新 group 节点列表，刷新本地 shard_clients
    /// - 保留仍然存在的连接
    /// - 添加新连接
    /// - 清除已失效连接
    pub async fn init_shard_kafka_list(&mut self) -> anyhow::Result<()> {
        log::info!("[ArbClient] Fetching current group node list...");

        self.shard_kafka_list.clear();
        // 获取最新仲裁节点列表
        let req = QueryNodeReq {
            node_type: NodeType::SocketNode as i32,
        };

        let resp = self.arb_client.list_all_nodes(req).await.map_err(|e| anyhow::anyhow!("Failed to list group nodes: {}", e))?.into_inner();

        let new_addrs: HashSet<String> =
            resp.nodes.iter().filter(|n| n.node_type == NodeType::GroupNode as i32).map(|n| n.node_addr.clone()).collect();

        //重新排序 list_addrs
        for node_addr in resp.nodes.iter() {
            let addr = &node_addr.kafka_addr.clone().unwrap();
            let kafka_service = KafkaGroupService::new(addr).await?;
            self.shard_kafka_list.insert(hash_index(addr, resp.nodes.len() as i32), kafka_service);
        }
        Ok(())
    }
    pub fn start_heartbeat(arb: Arc<RwLock<ArbClient>>) {
        let arb_clone = arb.clone();

        tokio::spawn(async move {
            let node_addr;
            let mut client;
            let guard = arb_clone.read().await;
            client = guard.arb_client.clone();
            node_addr = guard.node_addr.clone();
            let kafka_addr = guard.kafka_addr.clone();

            let mut interval = interval(Duration::from_secs(5));
            loop {
                interval.tick().await;
                let req = BaseRequest {
                    node_addr: node_addr.clone(),
                    node_type: NodeType::SocketNode as i32,
                };
                match client.heartbeat(req).await {
                    Ok(_) => log::info!("[Heartbeat] Sent heartbeat for node: {}", node_addr),
                    Err(err) => log::error!("[Heartbeat] Failed: {}", err),
                }
            }
        });
    }
}

static INSTANCE: OnceCell<Arc<RwLock<ArbClient>>> = OnceCell::new();
