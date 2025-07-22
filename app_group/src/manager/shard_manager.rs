use crate::protocol::rpc_arb_models::ShardState;
use anyhow::Result;
use arc_swap::ArcSwap;
use biz_service::protocol::msg::group_models::{ChangeGroupMsg, ChangeMemberRoleMsg, CreateGroupMsg, DestroyGroupMsg, ExitGroupMsg, HandleInviteMsg, HandleJoinRequestMsg, InviteMembersMsg, MemberOnlineMsg, MuteMemberMsg, RemoveMembersMsg, RequestJoinGroupMsg, TransferOwnershipMsg, UpdateMemberProfileMsg};
use common::config::{AppConfig, ShardConfig};
use common::util::common_utils::hash_index;
use common::{GroupId, UserId};
use dashmap::{DashMap, DashSet};
use once_cell::sync::OnceCell;
use std::collections::HashMap;
use std::{clone, format};
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::transport::Channel;
use tonic::{Status, async_trait};
use twox_hash::XxHash64;
use crate::protocol::rpc_arb_group::arb_group_service_client::ArbGroupServiceClient;

pub const GROUP_SHARD_SIZE: usize = 16;
pub const MEMBER_SHARD_SIZE: usize = 8;
pub struct GroupMembersPage {
    pub total: usize,
    pub members: Vec<UserId>,
}

#[derive(Debug, Default, Clone)]
pub struct ShardInfo {
    pub version: u64,
    pub state: ShardState,
    pub last_update_time: u64,
    pub last_heartbeat: u64,
    pub total: i32,
    pub index: i32,
    pub kafka_addr: String,
}
#[derive(Debug)]
pub struct MemData {
    pub group_shard_map: DashMap<String, DashMap<GroupId, ()>>,
    pub group_member_map: DashMap<String, DashMap<GroupId, Vec<DashSet<UserId>>>>, // ⬅️ 每个 group 成员可分片
    pub group_online_member_map: DashMap<String, DashMap<GroupId, DashSet<UserId>>>,
    pub shard_info: RwLock<ShardInfo>,
}

#[derive(Debug)]
pub struct ShardManager {
    //快速分片信息
    pub snapshot: ArcSwap<MemData>,
    //当前快片信息
    // pub current: Arc<RwLock<ShardInfo>>,
    //分片配置
    pub shard_config: ShardConfig,
    // 当前分片数据
    pub current: ArcSwap<MemData>,
    pub shard_address: String,
}
#[async_trait]
pub trait ShardManagerOpt: Send + Sync {
    async fn load_from(&self)  -> anyhow::Result<()>;
    /// 添加用户到指定群组（自动根据 group_id 映射分片）
    fn add_user_to_group(&self, group_id: &GroupId, uid: &UserId);
    /// 移除用户从指定群组（自动根据 group_id 映射分片）
    fn remove_user_from_group(&self, group_id: &GroupId, uid: &UserId);
    /// 获取某个群组的所有成员 ID 列表
    fn get_users_for_group(&self, group_id: &GroupId) -> Option<Vec<UserId>>;
    /// 获取群组成员分页列表
    fn get_group_members_page(
        &self,
        group_id: &GroupId,
        offset: usize,
        limit: usize,
    ) -> Option<Vec<UserId>>;

    fn get_group_member_total_count(&self, group_id: &GroupId) -> Option<usize>;
    /// 标记用户在线
    fn mark_user_online(&self, group_id: &GroupId, uid: &UserId);
    /// 移除在线用户
    fn mark_user_offline(&self, group_id: &GroupId, uid: &UserId);
    /// 获取某个群组的所有在线用户
    fn get_online_users_for_group(&self, group_id: &GroupId) -> Vec<UserId>;

    /// 获取在线管理员
    async fn get_admin_for_group(&self, group_id: &GroupId) -> Result<Option<Vec<UserId>>>;
}

#[async_trait]
pub trait ShardManagerMqOpt: Send + Sync {

    /// 创建群组
    async fn create_group(&self, group_id: &GroupId) -> anyhow::Result<()>;

    /// 解散群组
    async fn destroy_group(&self, msg: &DestroyGroupMsg) -> anyhow::Result<()>;

    /// 更新群组信息
    async fn change_group(&self, msg: &ChangeGroupMsg) -> anyhow::Result<()>;

    /// 用户申请加入群组
    async fn request_join_group(&self, msg: &RequestJoinGroupMsg) -> anyhow::Result<()>;

    /// 管理员处理加群请求
    async fn handle_join_request(&self, msg: &HandleJoinRequestMsg) -> anyhow::Result<()>;

    /// 邀请成员入群
    async fn invite_members(&self, msg: &InviteMembersMsg) -> anyhow::Result<()>;

    /// 受邀成员处理邀请
    async fn handle_invite(&self, msg: &HandleInviteMsg) -> anyhow::Result<()>;

    /// 移除成员
    async fn remove_members(&self, msg: &RemoveMembersMsg) -> anyhow::Result<()>;

    /// 成员主动退出群组
    async fn exit_group(&self, msg: &ExitGroupMsg) -> anyhow::Result<()>;

    /// 修改成员角色
    async fn change_member_role(&self, msg: &ChangeMemberRoleMsg) -> anyhow::Result<()>;

    /// 禁言或取消禁言成员
    async fn mute_member(&self, msg: &MuteMemberMsg) -> anyhow::Result<()>;

    /// 更新成员资料（群昵称/头像）
    async fn update_member_profile(&self, msg: &UpdateMemberProfileMsg) -> anyhow::Result<()>;

    /// 转让群主身份
    async fn transfer_owner_ship(&self, msg: &TransferOwnershipMsg) -> anyhow::Result<()>;
    
    async fn member_online(&self, msg:&MemberOnlineMsg) -> anyhow::Result<()>;
    
    async fn member_offline(&self, msg:&MemberOnlineMsg) -> anyhow::Result<()>;
}

impl ShardManager {
    pub fn new(shard_config: ShardConfig) -> Self {
        let shard_info = shard_config.clone();
        let mut info = ShardInfo::default();
        info.state = ShardState::Registered;
        let  manager=Self {
            snapshot: ArcSwap::new(Arc::new(MemData {
                group_shard_map: Default::default(),
                group_member_map: Default::default(),
                group_online_member_map: Default::default(),
                shard_info: RwLock::new(info.clone()),
            })),
            shard_config: shard_info,
            current: ArcSwap::new(Arc::new(MemData {
                group_shard_map: Default::default(),
                group_member_map: Default::default(),
                group_online_member_map: Default::default(),
                shard_info: RwLock::new(info.clone()),
            })),
            shard_address: shard_config.shard_address.clone().unwrap_or_default(),
        };
        return manager
    }
    pub fn hash_group_id(&self, group_id: &str) -> usize {
        use std::hash::{Hash, Hasher};
        use twox_hash::XxHash64;

        let mut hasher = XxHash64::with_seed(0);
        group_id.hash(&mut hasher);
        (hasher.finish() as usize) % GROUP_SHARD_SIZE
    }

    /// 计算群组成员索引（用于 group 成员缓存定位）
    pub fn hash_group_member_id(&self, group_id: &str, user_id: &str) -> usize {
        let mut hasher = XxHash64::with_seed(0);
        group_id.hash(&mut hasher);
        user_id.hash(&mut hasher);
        (hasher.finish() as usize) % MEMBER_SHARD_SIZE
    }
    pub fn get_node_addr(&self) -> &str {
        self.shard_config
            .shard_address
            .as_deref()
            .expect("shard_address must be set")
    }
    pub async fn init_grpc_clients(
        &self,
        endpoints: Vec<String>,
    ) -> std::result::Result<HashMap<i32, ArbGroupServiceClient<Channel>>, Box<dyn std::error::Error>>
    {
        let mut clients = HashMap::new();
        let size = endpoints.len();
        for endpoint in endpoints {
            //跳过自动节点
            if endpoint == self.shard_config.shard_address.clone().unwrap() {
                continue;
            }
            let channel = Channel::from_shared(format!("http://{}", endpoint))?
                .connect()
                .await?;
            let client = ArbGroupServiceClient::new(channel);
            clients.insert(hash_index(&endpoint, size as i32), client);
        }
        Ok(clients)
    }
    pub fn clear_current(&self) {
        self.current.store(Arc::new(MemData {
            group_shard_map: Default::default(),
            group_member_map: Default::default(),
            group_online_member_map: Default::default(),
            shard_info: RwLock::new(ShardInfo::default()),
        }));
    }

    pub fn clone_current_to_snapshot(&self) {
        let current = self.current.load();
        self.snapshot.store(current.clone());
    }
    pub fn clean_snapshot(&self) {
        self.snapshot.store(Arc::new(MemData {
            group_shard_map: Default::default(),
            group_member_map: Default::default(),
            group_online_member_map: Default::default(),
            shard_info: RwLock::new(ShardInfo::default()),
        }));
    }

    pub async fn init() {
        let app_cfg = AppConfig::get();
        let instance = Self::new(app_cfg.shard.clone().unwrap());
        instance.load_from().await.expect("Failed to load shard data");
        INSTANCE_COUNTRY
            .set(Arc::new(instance))
            .expect("INSTANCE already initialized");
    }

    /// 获取单例
    pub fn get() -> Arc<Self> {
        INSTANCE_COUNTRY
            .get()
            .expect("INSTANCE is not initialized")
            .clone()
    }
}
static INSTANCE_COUNTRY: OnceCell<Arc<ShardManager>> = OnceCell::new();
