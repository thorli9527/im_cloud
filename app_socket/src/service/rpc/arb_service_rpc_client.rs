// 在 app_socket 中添加节点注册与心跳逻辑到 app_arb 服务

use std::clone;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use config::Config;
use once_cell::sync::OnceCell;
use tokio::sync::RwLock;
use tonic::transport::Channel;
use crate::protocol::rpc_arb_models::{BaseRequest, NodeType, QueryNodeReq, ShardNodeInfo};
use tokio::time::{interval, Duration};
use biz_service::manager::group_manager_core::GroupManager;
use common::config::{AppConfig, ShardConfig};
use common::util::common_utils::hash_index;
use crate::protocol::rpc_arb_group::arb_group_service_client::ArbGroupServiceClient;
use crate::protocol::rpc_arb_server::arb_server_rpc_service_client::ArbServerRpcServiceClient;

#[derive(Debug, Clone)]
pub struct ArbClient {
    pub arb_client: ArbServerRpcServiceClient<Channel>,
    /// 仲裁子服务客户端集合（Key = shard_address，例如 "10.0.0.12:50051"）
    /// 每个地址对应一个独立的 `ArbGroupServiceClient<Channel>` 实例。
    pub shard_clients: HashMap<String, ArbGroupServiceClient<Channel>>,
    ///当前节点地址
    pub node_addr: String,
}

impl ArbClient {
    /// 创建 ArbClient 并注册自身地址
    pub async fn new(shard_config: &ShardConfig) -> anyhow::Result<Self> {
        let node_addr = shard_config
            .server_host
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Missing server_host in ShardConfig"))?;

        let client = ArbServerRpcServiceClient::connect(node_addr.clone()).await?;

        Ok(Self {
            arb_client: client,
            shard_clients: HashMap::new(),
            node_addr,
        })
    }
    pub fn get() -> Arc<RwLock<ArbClient>> {
        INSTANCE
            .get()
            .expect("ArbClient is not initialized")
            .clone()
    }
    /// 注册本节点 + 初始化 shard_clients
    pub async fn init() -> anyhow::Result<()> {
        let shard_config = AppConfig::get().shard.clone().unwrap();
        let mut arb_client = ArbClient::new(&shard_config).await?;


         arb_client.register().await?;

        // 初始化 shard clients 列表
        arb_client.init_shard_clients().await?;

        log::info!(
            "[ArbClient] Init complete: shard_clients = {}",
            arb_client.shard_clients.len()
        );
        INSTANCE.set(Arc::new(RwLock::new(arb_client))).expect("ArbClient init failed");
        Ok(())
    }
    pub fn get_shard_client(&self, group_id:&str,shard_id: u32) -> anyhow::Result<ArbGroupServiceClient<Channel>> {
        let shard_group_index=hash_index(group_id,self.shard_clients.len() as i32);
        for (node_addr,client) in &self.shard_clients  {
            let shard_node_index=hash_index(&node_addr,self.shard_clients.len() as i32);
            if shard_node_index==shard_group_index {
                return Ok(client.clone());
            }
        }
        Err(anyhow::anyhow!("no shard node"))
    }
    pub async fn register(&mut self) -> anyhow::Result<ShardNodeInfo> {
        let req = BaseRequest {
            node_addr: self.node_addr.clone(),
            node_type: NodeType::SocketNode as i32,
        };
        let resp = self.arb_client.register_node(req).await?.into_inner();
        Ok(resp)
    }

    /// 根据仲裁服务返回的最新 group 节点列表，刷新本地 shard_clients
    /// - 保留仍然存在的连接
    /// - 添加新连接
    /// - 清除已失效连接
    pub async fn init_shard_clients(&mut self) -> anyhow::Result<()> {
        log::info!("[ArbClient] Fetching current group node list...");

        // 获取最新仲裁节点列表
        let req = QueryNodeReq {
            node_type: NodeType::GroupNode as i32,
        };

        let resp = self
            .arb_client
            .list_all_nodes(req)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list group nodes: {}", e))?
            .into_inner();

        let new_addrs: HashSet<String> = resp.nodes
            .iter()
            .filter(|n| n.node_type == NodeType::GroupNode as i32)
            .map(|n| n.node_addr.clone())
            .collect();

        let old_addrs: HashSet<String> = self.shard_clients.keys().cloned().collect();

        // === 1. 移除已不存在的节点 ===
        for addr in old_addrs.difference(&new_addrs) {
            self.shard_clients.remove(addr);
            log::info!("[ArbClient] Removed stale group client: {}", addr);
        }

        // === 2. 添加新节点连接 ===
        for addr in new_addrs.difference(&old_addrs) {
            match Channel::from_shared(format!("http://{}", addr))?
                .connect()
                .await
            {
                Ok(channel) => {
                    let client = ArbGroupServiceClient::new(channel);
                    self.shard_clients.insert(addr.clone(), client);
                    log::info!("[ArbClient] Added new group client: {}", addr);
                }
                Err(err) => {
                    log::warn!("[ArbClient] Failed to connect to {}: {}", addr, err);
                }
            }
        }

        // === 3. 打印汇总 ===
        log::info!(
            "[ArbClient] Shard client sync complete. Total active: {}",
            self.shard_clients.len()
        );

        Ok(())
    }
    pub fn start_heartbeat(arb: Arc<RwLock<ArbClient>>) {
        let arb_clone = arb.clone();
        tokio::spawn(async move {
            let node_addr;
            let mut client;
            {
                let guard = arb_clone.read().await;
                client = guard.arb_client.clone();
                node_addr = guard.node_addr.clone();
            }
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