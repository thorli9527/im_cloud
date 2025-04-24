use crate::manager::socket_error::SendError;
use crate::pb::protocol::DeviceType;
use dashmap::DashMap;
use log::warn;
use once_cell::sync::OnceCell;
use prost::bytes::Bytes;
use prost::Message;
use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};

#[derive(Clone)]
pub struct ConnectionMeta {
    pub user_id: Option<String>,
    pub client_id: Option<String>,
    pub device_type: Option<DeviceType>,
}

#[derive(Clone)]
pub struct ConnectionInfo {
    pub meta: ConnectionMeta,
    pub sender: mpsc::UnboundedSender<Bytes>,
    pub last_heartbeat: Arc<AtomicU64>,
}
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct ConnectionId(pub String);
pub struct SocketManager {
    pub connections: DashMap<ConnectionId, ConnectionInfo>,
    pub user_index:  DashMap<String, HashSet<ConnectionId>>,
    pub group_members: DashMap<String, HashSet<String>>,
}


impl SocketManager {
    pub fn new() -> Self {
        Self {
            connections: DashMap::new(),user_index:DashMap::new(),group_members:DashMap::new()
        }
    }

    pub fn insert(&self, id: impl Into<ConnectionId>, conn: ConnectionInfo) {
        self.connections.insert(id.into(), conn);
    }

    pub fn remove(&self, id: &ConnectionId) {
        if let Some((_, conn)) = self.connections.remove(id) {
            if let Some(user_id) = &conn.meta.user_id {
                if let Some(mut set) = self.user_index.get_mut(user_id) {
                    set.remove(id);
                    // 可选：连接为空时清理 user_id 条目
                    if set.is_empty() {
                        drop(set); // 释放引用后移除
                        self.user_index.remove(user_id);
                    }
                }
            }
        }
    }

    pub fn get(&self, id: &ConnectionId) -> Option<ConnectionInfo> {
        self.connections.get(id).map(|v| v.clone())
    }

    fn update_heartbeat(info: &ConnectionInfo) {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        info.last_heartbeat.store(now, Ordering::Relaxed);
    }
    pub fn send_client<M: Message>(
        &self,
        id: &ConnectionId,
        bytes: Bytes,
    ) -> Result<(), SendError> {
        let conn = self.connections.get(id).ok_or(SendError::ConnectionNotFound)?;
        conn.sender
            .send(bytes)
            .map_err(|_| SendError::ChannelClosed)?;

        Ok(())
    }
    pub fn get_connections_by_user(&self, user_id: &str) -> Vec<ConnectionInfo> {
        self.user_index
            .get(user_id)
            .map(|set| {
                set.iter()
                    .filter_map(|conn_id| self.connections.get(conn_id).map(|c| c.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn send_to_connection(
        &self,
        id: &ConnectionId,
        bytes: Bytes,
    ) -> Result<(), SendError> {
        let conn = self.connections.get(id).ok_or(SendError::ConnectionNotFound)?;
        conn.sender
            .send(bytes)
            .map_err(|_| SendError::ChannelClosed)?;
        Ok(())
    }

    pub fn send_to_user(
        &self,
        user_id: &str,
        bytes: Bytes,
        device_type: Option<DeviceType>,
    ) -> Result<(), SendError> {
        let mut sent = false;

        // 获取该用户的所有连接 ID
        if let Some(conn_ids) = self.user_index.get(user_id) {
            // 先编码 protobuf 消息
            for conn_id in conn_ids.iter() {
                if let Some(conn) = self.connections.get(conn_id) {
                    // 如果指定了设备类型，则做筛选
                    if device_type.is_none()
                        || conn.meta.device_type.as_ref() == device_type.as_ref()
                    {
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
            warn!("用户不在线: {:?}", user_id);
            Ok(())
        }
    }
}

static SOCKET_MANAGER: OnceCell<Arc<SocketManager>> = OnceCell::new();

pub fn get_socket_manager() -> Arc<SocketManager> {
    SOCKET_MANAGER.get_or_init(|| Arc::new(SocketManager::new())).clone()
}