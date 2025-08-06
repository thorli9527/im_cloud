use crate::service::shard_manager::ShardManager;
use async_trait::async_trait;
use biz_service::protocol::rpc::arb_models::ShardState::{Migrating, Normal, Preparing, Ready, Registered};
use biz_service::protocol::rpc::arb_server::arb_server_rpc_service_client::ArbServerRpcServiceClient;
use common::config::AppConfig;
use once_cell::sync::OnceCell;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::sleep;
use tonic::transport::Channel;

#[derive(Debug)]
pub struct ArbManagerJob {
    pub arb_client: Option<ArbServerRpcServiceClient<Channel>>,
    pub server_host: String,
    pub shard_address: String,
    pub total: usize,
}

#[async_trait]
/// 分片管理器核心操作定义接口，适用于支持动态迁移、健康上报、状态切换的分布式群组服务。
pub trait ManagerJobOpt: Send + Sync {
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

impl ArbManagerJob {
    pub fn new() -> Self {
        let config1 = &AppConfig::get().clone().shard.clone().unwrap();
        Self {
            arb_client: None,
            server_host: config1.server_addr.clone().unwrap().clone(),
            shard_address: config1.client_addr.clone().unwrap().clone(),
            total: 0,
        }
    }

    /// 启动心跳和生命周期任务
    pub async fn start(&mut self) -> () {
        self.start_heartbeat_loop();
        self.check_status_task().await;
    }
    /// 启动分片状态自检任务（后台常驻）
    pub async fn check_status_task(&mut self) {
        loop {
            sleep(Duration::from_secs(15)).await;
            let shard_manager = ShardManager::get();
            let current = shard_manager.current.load();
            let shard_info = current.shard_info.read().await;

            if shard_info.state == Registered {
                if let Err(e) = self.register_node().await {
                    log::error!("register_node error: {:?}", e);
                }
            }
            if shard_info.state == Migrating {
                if let Err(e) = self.change_migrating().await {
                    log::error!("change_migrating error: {:?}", e);
                }
            }
            if shard_info.state == Preparing {
                if let Err(e) = self.change_preparing().await {
                    log::error!("Preparing error: {:?}", e);
                }
            }
            if shard_info.state == Ready {
                if let Err(e) = self.change_ready().await {
                    log::error!("Ready error: {:?}", e);
                }
            }
            if shard_info.state == Normal {
                if let Err(e) = self.change_normal().await {
                    log::error!("Normal error: {:?}", e);
                }
            }
        }
    }
    /// 停止所有任务
    pub fn stop(&self) {}
    pub async fn init_arb_client(&mut self) -> anyhow::Result<&mut ArbServerRpcServiceClient<Channel>> {
        if self.arb_client.is_none() {
            let node_addr = self.server_host.clone();
            let client = ArbServerRpcServiceClient::connect(format!("http://{}", node_addr)).await?;
            self.arb_client = Some(client);
        }
        Ok(self.arb_client.as_mut().unwrap())
    }

    fn start_heartbeat_loop(&mut self) {
        let mut this = self.clone_light();
        Some(tokio::spawn(async move {
            let interval = std::time::Duration::from_secs(10);
            loop {
                tokio::select! {
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
    pub fn clone_light(&self) -> ArbManagerJob {
        ArbManagerJob {
            arb_client: None, // 避免 tonic 客户端跨线程问题
            server_host: self.server_host.clone(),
            shard_address: self.shard_address.clone(),
            total: 0,
        }
    }

    pub async fn init() -> anyhow::Result<()> {
        let mut job = Self::new();
        job.init_arb_client().await?;
        INSTANCE_COUNTRY.set(Arc::new(job)).expect("INSTANCE already initialized");
        Ok(())
    }

    /// 获取单例
    pub fn get() -> Arc<Self> {
        INSTANCE_COUNTRY.get().expect("INSTANCE is not initialized").clone()
    }
}
static INSTANCE_COUNTRY: OnceCell<Arc<ArbManagerJob>> = OnceCell::new();
