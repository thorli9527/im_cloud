use crate::manager;
use crate::manager::shard_manager;
use crate::manager::shard_manager::{GROUP_SHARD_SIZE, ShardManager};
use crate::protocol::rpc_arb_server::arb_server_rpc_service_client::ArbServerRpcServiceClient;
use crate::service::rpc::group_rpc_service_impl::GroupRpcServiceImpl;
use common::config::ShardConfig;
use common::util::common_utils::hash_index;
use common::{GroupId, UserId};
use dashmap::{DashMap, DashSet};
use std::collections::HashMap;
use std::sync::Arc;
use std::thread::current;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tonic::async_trait;
use tonic::transport::Channel;
use crate::protocol::rpc_arb_group::arb_group_service_client::ArbGroupServiceClient;
use crate::protocol::rpc_arb_group::arb_group_service_server::ArbGroupServiceServer;

pub struct ManagerJob {
    //分片仲裁服务器接口信息
    pub arb_client: Option<ArbServerRpcServiceClient<Channel>>,
    pub shard_address: String,
    pub server_host: String,
    pub cancel_token: CancellationToken,
    pub heartbeat_handle: Option<JoinHandle<()>>,
}
impl ManagerJob {
    pub fn new(shard_config: ShardConfig) -> Self {
        Self {
            arb_client: None,
            shard_address: shard_config.shard_address.unwrap(),
            server_host: shard_config.server_host.unwrap(),
            cancel_token: CancellationToken::new(),
            heartbeat_handle: None,
        }
    }

    /// 启动心跳和生命周期任务
    pub async fn start(&mut self) -> () {
        self.register_node().await.expect("register node error");
        self.change_preparing().await.expect("change preparing error");
        self.change_migrating().await.expect("change migrating error");
        self.sync_data().await.expect("sync groups error");
        self.change_ready().await.expect("change ready error");
        self.change_normal().await.expect("change normal error");
        self.start_heartbeat_loop();
    }

    /// 停止所有任务
    pub fn stop(&self) {}
    pub async fn init_arb_client(
        &mut self,
    ) -> anyhow::Result<&mut ArbServerRpcServiceClient<Channel>> {
        if self.arb_client.is_none() {
            let client = ArbServerRpcServiceClient::connect(self.server_host.clone()).await?;
            self.arb_client = Some(client);
        }
        Ok(self.arb_client.as_mut().unwrap())
    }
    pub async fn init_grpc_clients(&self,
        endpoints: Vec<String>,
    ) -> Result<HashMap<i32, ArbGroupServiceClient<Channel>>, Box<dyn std::error::Error>> {
        let mut clients = HashMap::new();
        let size = endpoints.len();
        for endpoint in endpoints {
            let channel = Channel::from_shared(endpoint.clone())?.connect().await?;

            let client = ArbGroupServiceClient::new(channel);
            clients.insert(hash_index(&endpoint, size as i32), client);
        }
        Ok(clients)
    }

    fn start_heartbeat_loop(&mut self) {
        let cancel_token = self.cancel_token.clone();
        let mut this = self.clone_light(); // 👈 克隆必要字段以避免借用冲突

        self.heartbeat_handle = Some(tokio::spawn(async move {
            let interval = std::time::Duration::from_secs(10);
            loop {
                tokio::select! {
                    _ = cancel_token.cancelled() => {
                        log::info!("🛑 心跳任务已取消");
                        break;
                    }
                    _ = tokio::time::sleep(interval) => {
                        if let Err(e) = this.heartbeat().await {
                            log::warn!("⚠️ 心跳失败: {:?}", e);
                        } else {
                            log::debug!("❤️ 心跳发送成功");
                        }
                    }
                }
            }
        }));
    }

    // 轻量 clone，只克隆非连接字段
    fn clone_light(&self) -> ManagerJob {
        ManagerJob {
            arb_client: None, // 避免 tonic 客户端跨线程问题
            shard_address: self.shard_address.clone(),
            server_host: self.server_host.clone(),
            cancel_token: self.cancel_token.clone(),
            heartbeat_handle: None,
        }
    }
}

#[async_trait]
/// 分片管理器核心操作定义接口，适用于支持动态迁移、健康上报、状态切换的分布式群组服务。
pub trait ManagerJobOpt: Send + Sync {
    /// 初始化管理器（例如加载缓存、连接仲裁器、预拉取分片信息等）
    async fn init(&mut self) -> anyhow::Result<()>;

    /// 注册当前节点到仲裁中心或注册服务，用于初始接入和负载调度识别
    async fn register_node(&mut self) -> anyhow::Result<()>;
    /// 设置某群组迁移状态为“准备中”
    /// 表示目标节点已准备好接收群组（例如缓存准备、校验完成等）
    async fn change_preparing(&mut self) -> anyhow::Result<()>;
    /// 将群组分片状态设置为“迁移中”
    /// 通常意味着不再接受新写入，同时准备数据转移
    async fn change_migrating(&mut self) -> anyhow::Result<()>;
    /// 同步当前群组列表（通常从仲裁服务或中心节点拉取最新群组分配情况）
    async fn sync_data(&mut self) -> anyhow::Result<()>;
    /// 设置群组状态为“迁移失败”
    /// 可用于回滚操作或触发异常迁移重试逻辑
    async fn change_failed(&mut self) -> anyhow::Result<()>;

    /// 设置为“就绪”状态，表示目标节点已接管数据并可激活群组
    async fn change_ready(&mut self) -> anyhow::Result<()>;

    /// 设置为“正常”状态，表示群组已完成迁移并稳定运行
    async fn change_normal(&mut self) -> anyhow::Result<()>;
    ///准备下线
    async fn change_preparing_offline(&mut self) -> anyhow::Result<()>;
    /// 节点下线
    async fn change_offline(&mut self) -> anyhow::Result<()>;
    /// 向仲裁服务上报心跳信息（包括负载、分片列表等），用于节点健康检查
    async fn heartbeat(&mut self) -> anyhow::Result<()>;
}
