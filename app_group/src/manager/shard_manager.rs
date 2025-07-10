use std::collections::HashMap;
use common::config::ShardConfig;
use anyhow::Result;
use tonic::codegen::http::status;
use tonic::Status;
use tonic::transport::Channel;

#[derive(Debug)]
pub struct ShardManager {
    pub shard_config: ShardConfig,
    pub arb_client: Option<ArbiterServiceClient<Channel>>,
    pub vnode_versions: HashMap<u64, u64>,
}


impl ShardManager {
    pub fn new(shard_config: ShardConfig) -> Self {
        Self {
            shard_config,
            arb_client: None,
            vnode_versions: HashMap::new(),
        }
    }

    pub async fn client_init(&mut self) -> Result<&mut ArbiterServiceClient<Channel>> {
        if self.arb_client.is_none() {
            let cli = ArbiterServiceClient::connect("http://[::1]:50051").await?;
            self.arb_client = Some(cli);
        }
        Ok(self.arb_client.as_mut().unwrap())
    }
}
#[async_trait]
/// 分片管理器核心操作定义接口，适用于支持动态迁移、健康上报、状态切换的分布式群组服务。
pub trait ShardManagerOpt: Send + Sync {
    /// 初始化管理器（例如加载缓存、连接仲裁器、预拉取分片信息等）
    async fn init(&mut self) -> Result<()> ;

    /// 注册当前节点到仲裁中心或注册服务，用于初始接入和负载调度识别
    async fn register_node(&mut self, node_addr: &str) -> Result<()>;

    /// 将群组分片状态设置为“迁移中”
    /// 通常意味着不再接受新写入，同时准备数据转移
    async fn change_migrating(&mut self) -> Result<()>;

    /// 同步当前群组列表（通常从仲裁服务或中心节点拉取最新群组分配情况）
    async fn sync_groups(&mut self) -> Result<()>;

    /// 同步群组成员列表信息（确保迁移前/后成员视图一致）
    async fn sync_group_members(&mut self) -> Result<()>;

    /// 设置某群组迁移状态为“准备中”
    /// 表示目标节点已准备好接收群组（例如缓存准备、校验完成等）
    async fn change_preparing(&mut self) -> Result<()>;

    /// 设置群组状态为“迁移失败”
    /// 可用于回滚操作或触发异常迁移重试逻辑
    async fn change_failed(&mut self) -> Result<()>;

    /// 设置为“就绪”状态，表示目标节点已接管数据并可激活群组
    async fn change_ready(&mut self) -> Result<()>;

    /// 设置为“正常”状态，表示群组已完成迁移并稳定运行
    async fn change_normal(&mut self) -> Result<()>;
    ///准备下线
    async fn change_preparing_offline(&mut self) -> Result<()>;
    /// 节点下线
    async fn change_offline(&mut self) -> Result<()>;
    /// 向仲裁服务上报心跳信息（包括负载、分片列表等），用于节点健康检查
    async fn heartbeat(&mut self) -> Result<()>;
}
include!("shard_manager_impl.rs");