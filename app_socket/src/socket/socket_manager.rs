use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::socket::socket_error::SendError;
use anyhow::Result;
use biz_service::protocol::common::{ByteMessageType, ChatTargetType};
use biz_service::protocol::msg::auth::DeviceType;
use biz_service::protocol::rpc::arb_models::NodeInfo;
use common::config::AppConfig;
use common::util::common_utils::hash_index;
use common::UserId;
use dashmap::DashMap;
use log::{info, warn};
use once_cell::sync::OnceCell;
use prost::bytes::Bytes;
use prost::Message;
use tokio::sync::mpsc;
static MESSAGE_CACHE: OnceCell<Arc<DashMap<MessageId, (CachedResponse, u64)>>> = OnceCell::new();

type MessageId = u64;
type CachedResponse = Bytes;

/// 客户端连接唯一标识
#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct ConnectionId(pub String);
#[derive(Clone, Debug)]
pub enum FocusTarget {
    User(Arc<str>),
    Group(Arc<str>),
}
/// 连接元信息（用户、设备、客户端等）
#[derive(Clone)]
pub struct ConnectionMeta {
    pub uid: Option<UserId>,
    pub device_type: Option<DeviceType>,
}

/// 连接实体（包含心跳状态）
#[derive(Clone)]
pub struct ConnectionInfo {
    pub meta: ConnectionMeta,
    pub sender: mpsc::UnboundedSender<Bytes>,
    pub last_heartbeat: Arc<AtomicU64>,
    pub focus_target: Option<FocusTarget>,
}

/// Socket连接管理器：用于统一管理所有在线连接、用户索引及群组关系
pub struct SocketManager {
    /// 所有客户端连接的主索引，键为连接唯一ID（ConnectionId），值为连接信息（ConnectionInfo）
    /// - 支持通过连接ID快速定位具体连接
    /// - 用于消息定向发送、连接关闭、心跳检测等
    pub connections: DashMap<ConnectionId, ConnectionInfo>,

    /// 用户ID到连接ID集合的映射
    /// - 支持一个用户拥有多个设备连接（多终端支持）
    /// - 用于向用户广播、判断用户是否在线
    /// - 格式: user_id → {conn_id1, conn_id2, ...}
    pub user_index: DashMap<String, HashSet<ConnectionId>>,

    /// 群组ID到用户ID集合的映射
    /// - 用于进行群组广播消息派发
    /// - 格式: group_id → {user_id1, user_id2, ...}
    pub group_members: DashMap<String, HashSet<String>>,
}

impl SocketManager {
    pub fn new() -> Self {
        Self {
            connections: DashMap::new(),
            user_index: DashMap::new(),
            group_members: DashMap::new(),
        }
    }

    pub fn get_message_cache(&self) -> Arc<DashMap<MessageId, (CachedResponse, u64)>> {
        MESSAGE_CACHE.get_or_init(|| Arc::new(DashMap::new())).clone()
    }

    /// 新增连接
    pub fn insert(&self, id: impl Into<ConnectionId>, conn: ConnectionInfo) {
        let id = id.into();
        if let Some(uid) = &conn.meta.uid {
            self.user_index.entry(uid.clone()).or_insert_with(HashSet::new).insert(id.clone());
        }
        self.connections.insert(id, conn);
    }

    /// 移除连接
    pub fn remove(&self, id: &ConnectionId) {
        if let Some((_, conn)) = self.connections.remove(id) {
            if let Some(user_id) = &conn.meta.uid {
                if let Some(mut set) = self.user_index.get_mut(user_id) {
                    set.remove(id);
                    if set.is_empty() {
                        drop(set);
                        self.user_index.remove(user_id);
                    }
                }
            }
            info!("🔌 连接断开: {:?}", id.0);
        }
    }
    /// 获取连接
    pub fn get_by_id(&self, conn_id: &ConnectionId) -> Option<Arc<ConnectionInfo>> {
        self.connections.get(conn_id).map(|v| Arc::new(v.clone()))
    }
    pub fn get() -> Arc<SocketManager> {
        get_socket_manager()
    }

    /// 心跳续期
    pub fn touch(info: &ConnectionInfo) {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        info.last_heartbeat.store(now, Ordering::Relaxed);
    }

    /// 发送到指定连接
    pub fn send_to_connection(&self, id: &ConnectionId, bytes: Bytes) -> Result<(), SendError> {
        let conn = self.connections.get(id).ok_or(SendError::ConnectionNotFound)?;
        conn.sender.send(bytes).map_err(|_| SendError::ChannelClosed)
    }
    ///发送消息到指定连接（使用 ByteMessageType 前缀）
    pub fn send_to_connection_proto<M: Message>(
        &self,
        message_id: &Option<u64>,
        id: &ConnectionId,
        msg_type: &ByteMessageType,
        msg: &M,
    ) -> Result<(), SendError> {
        let mut buf = Vec::with_capacity(128);
        buf.push(*msg_type as u8);
        msg.encode(&mut buf).map_err(|_| SendError::EncodeError)?;
        let bytes = Bytes::from(buf);

        if let Some(message_id) = message_id {
            let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
            self.get_message_cache().insert(*message_id, (bytes.clone(), now));
        }

        self.send_to_connection(id, bytes)
    }

    pub fn send_to_user_proto<M: Message>(
        &self,
        user_id: &str,
        msg_type: &ByteMessageType,
        msg: &M,
        device_filter: Option<DeviceType>,
    ) -> Result<(), SendError> {
        let mut buf = Vec::with_capacity(128);
        buf.push(*msg_type as u8);
        msg.encode(&mut buf).map_err(|_| SendError::EncodeError)?;
        self.send_to_user(user_id, Bytes::from(buf), device_filter)
    }
    /// 获取用户所有连接
    pub fn get_connections_by_user(&self, user_id: &str) -> Vec<ConnectionInfo> {
        self.user_index.get(user_id).map(|set| set.iter().filter_map(|id| self.connections.get(id).map(|c| c.clone())).collect()).unwrap_or_default()
    }

    /// 向用户发送消息（支持设备类型过滤）
    pub fn send_to_user(&self, user_id: &str, bytes: Bytes, device_filter: Option<DeviceType>) -> Result<(), SendError> {
        let mut sent = false;

        if let Some(conn_ids) = self.user_index.get(user_id) {
            for conn_id in conn_ids.iter() {
                if let Some(conn) = self.connections.get(conn_id) {
                    if device_filter.map_or(true, |d| conn.meta.device_type == Some(d)) {
                        if conn.sender.send(bytes.clone()).is_ok() {
                            sent = true;
                        }
                    }
                }
            }
        }

        if sent {
            Ok(())
        } else {
            warn!("📭 用户不在线或设备不匹配: {}", user_id);
            Ok(())
        }
    }

    /// 群组广播（预留）
    pub fn send_to_group(&self, group_id: &str, bytes: Bytes) -> Result<(), SendError> {
        let mut sent = false;

        if let Some(users) = self.group_members.get(group_id) {
            for user_id in users.iter() {
                let _ = self.send_to_user(user_id, bytes.clone(), None)?;
                sent = true;
            }
        }

        if sent {
            Ok(())
        } else {
            warn!("📭 群组无在线用户: {}", group_id);
            Ok(())
        }
    }
    /// 返回所有连接快照 (conn_id, conn_info)
    pub fn all_connections(&self) -> Vec<(ConnectionId, ConnectionInfo)> {
        self.connections.iter().map(|entry| (entry.key().clone(), entry.value().clone())).collect()
    }
    /// 检查所有连接是否需要迁移，若不属于当前节点，则发送断开通知
    pub async fn dispatch_mislocated_connections(socket_list: Vec<NodeInfo>) -> Result<()> {
        let manager = get_socket_manager();
        let connections = manager.all_connections(); // snapshot

        let node_count = socket_list.len();
        if node_count == 0 {
            log::warn!("⚠️ socket_list 为空，跳过连接迁移检查");
            return Ok(());
        }
        let socket_list = sort_nodes(socket_list);
        let socket_addr = AppConfig::get().get_shard().server_addr.unwrap_or_default();
        for (conn_id, conn_info) in connections {
            let idx = hash_index(&conn_id.0, node_count as i32);

            match socket_list.get(idx as usize) {
                Some(target_node) if target_node.node_addr != socket_addr => {
                    log::info!("🚧 连接不属于本节点，迁移中: conn_id={:?}, 分配节点={}", conn_id.0, target_node.node_addr,);

                    // 可使用 RECONNECT 消息结构替代裸字符串
                    let _ = conn_info.sender.send(Bytes::from("RECONNECT"));

                    // 移除本地连接（客户端将自动重连）
                    manager.remove(&conn_id);
                }
                _ => {
                    // 属于当前节点，无需处理
                }
            }
        }

        log::info!("✅ 连接迁移检查完成，连接总数：{}", manager.connections.len());
        return Ok(());
    }
}

static SOCKET_MANAGER: OnceCell<Arc<SocketManager>> = OnceCell::new();

/// 获取全局 SocketManager 实例
pub fn get_socket_manager() -> Arc<SocketManager> {
    SOCKET_MANAGER.get_or_init(|| Arc::new(SocketManager::new())).clone()
}

// 对节点列表进行排序 按照地址
fn sort_nodes(mut nodes: Vec<NodeInfo>) -> Vec<NodeInfo> {
    nodes.sort_by(|a, b| a.node_addr.cmp(&b.node_addr));
    nodes
}
