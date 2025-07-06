use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::manager::socket_error::SendError;
use biz_service::protocol::auth::DeviceType;
use dashmap::DashMap;
use log::{info, warn};
use once_cell::sync::OnceCell;
use prost::bytes::Bytes;
use prost::Message;
use tokio::sync::mpsc;
use biz_service::protocol::common::ByteMessageType;

/// 客户端连接唯一标识
#[derive(Clone, Eq, PartialEq, Hash,Debug)]
pub struct ConnectionId(pub String);

/// 连接元信息（用户、设备、客户端等）
#[derive(Clone)]
pub struct ConnectionMeta {
    pub user_id: Option<String>,
    pub client_id: Option<String>,
    pub device_type: Option<DeviceType>,
}

/// 连接实体（包含心跳状态）
#[derive(Clone)]
pub struct ConnectionInfo {
    pub meta: ConnectionMeta,
    pub sender: mpsc::UnboundedSender<Bytes>,
    pub last_heartbeat: Arc<AtomicU64>,
}

/// Socket连接管理器：维护连接、分发消息、处理心跳
pub struct SocketManager {
    pub connections: DashMap<ConnectionId, ConnectionInfo>,
    pub user_index: DashMap<String, HashSet<ConnectionId>>,
    pub group_members: DashMap<String, HashSet<String>>, // 群组ID -> 用户ID列表
}

impl SocketManager {
    pub fn new() -> Self {
        Self { connections: DashMap::new(), user_index: DashMap::new(), group_members: DashMap::new() }
    }

    /// 新增连接
    pub fn insert(&self, id: impl Into<ConnectionId>, conn: ConnectionInfo) {
        let id = id.into();
        if let Some(user_id) = &conn.meta.user_id {
            self.user_index.entry(user_id.clone()).or_insert_with(HashSet::new).insert(id.clone());
        }
        self.connections.insert(id, conn);
    }

    /// 移除连接
    pub fn remove(&self, id: &ConnectionId) {
        if let Some((_, conn)) = self.connections.remove(id) {
            if let Some(user_id) = &conn.meta.user_id {
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
    pub fn get(&self, id: &ConnectionId) -> Option<ConnectionInfo> {
        self.connections.get(id).map(|v| v.clone())
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
    pub fn send_to_connection_proto<M: Message>(
        &self,
        id: &ConnectionId,
        msg_type: &ByteMessageType,
        msg: &M,
    ) -> Result<(), SendError> {
        let mut buf = Vec::with_capacity(128);
        buf.push(*msg_type as u8); // 消息类型前缀
        msg.encode(&mut buf).map_err(|_| SendError::EncodeError)?;
        self.send_to_connection(id, Bytes::from(buf))
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
        self.user_index
            .get(user_id)
            .map(|set| set.iter().filter_map(|id| self.connections.get(id).map(|c| c.clone())).collect())
            .unwrap_or_default()
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

}

static SOCKET_MANAGER: OnceCell<Arc<SocketManager>> = OnceCell::new();

/// 获取全局 SocketManager 实例
pub fn get_socket_manager() -> Arc<SocketManager> {
    SOCKET_MANAGER.get_or_init(|| Arc::new(SocketManager::new())).clone()
}
