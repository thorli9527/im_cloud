use crate::manager::socket_manager::{
    ConnectionId, ConnectionInfo, ConnectionMeta, get_socket_manager,
};
use anyhow::{Result, anyhow};
use biz_service::protocol::common::ByteMessageType;
use bytes::Buf;
use common::util::common_utils::build_uuid;
use common::util::date_util::now;
use futures::{SinkExt, StreamExt};
use prost::Message;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use rdkafka::types::RDKafkaApiKey::Heartbeat;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};
use biz_service::protocol::auth::AuthReqMsg;
use biz_service::protocol::entity::{GroupMsg, UserMsg};
use biz_service::protocol::friend::FriendEventMsg;
use biz_service::protocol::status::{AckMsg, HeartbeatMsg};
use biz_service::protocol::system::SystemNotificationMsg;
use crate::heartbeat_handler::{start_heartbeat_tasks, stop_heartbeat_tasks};

/// 客户端连接处理入口
pub async fn handle_connection(stream: TcpStream) -> Result<()> {
    let conn_id = build_uuid();
    let (read_half, write_half) = stream.into_split();
    let mut reader = FramedRead::new(read_half, LengthDelimitedCodec::new());
    let mut writer = FramedWrite::new(write_half, LengthDelimitedCodec::new());

    let (tx, mut rx) = mpsc::unbounded_channel();
    let last_heartbeat = Arc::new(AtomicU64::new(now() as u64));
    let conn_key = ConnectionId(conn_id.clone());

    let connection = ConnectionInfo {
        meta: ConnectionMeta {
            user_id: None,
            client_id: None,
            device_type: None,
        },
        sender: tx.clone(),
        last_heartbeat: last_heartbeat.clone(),
    };

    let manager = get_socket_manager();
    manager.insert(conn_key.clone(), connection);

    // 启动心跳相关任务（客户端超时检查）
    start_heartbeat_tasks(conn_key.clone(), last_heartbeat.clone());

    // 启动写任务
    let write_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Err(e) = writer.send(msg).await {
                log::warn!("❌ 写入客户端失败: {:?}", e);
                break;
            }
        }
    });

    // 启动读取任务
    let result = read_loop(
        &mut reader,
        conn_key.clone(),
        last_heartbeat.clone(),
    )
    .await;

    // 清理资源
    manager.remove(&conn_key);
    stop_heartbeat_tasks(&conn_key);
    write_task.abort();
    result
}

/// 读取客户端数据 & 处理消息
async fn read_loop(
    reader: &mut FramedRead<tokio::net::tcp::OwnedReadHalf, LengthDelimitedCodec>,
    conn_key: ConnectionId,
    last_heartbeat: Arc<AtomicU64>,
) -> Result<()> {
    while let Some(frame) = reader.next().await {
        let mut bytes = frame?;

        if bytes.len() < 1 {
            return Err(anyhow!("⚠️ 数据包长度不足"));
        }

        let type_code = bytes.get_u8();
        let message_type = ByteMessageType::from_i32(type_code as i32)
            .unwrap_or(ByteMessageType::UnknownByteMessageType);

        match message_type {
            ByteMessageType::AuthType => {
                let _ = AuthReqMsg::decode(bytes)?;
                log::info!("🛂 收到认证消息");
            }
            ByteMessageType::FriendType => {
                let _ = FriendEventMsg::decode(bytes)?;
                log::debug!("👥 好友事件处理");
            }
            ByteMessageType::UserMessageType => {
                let _ = UserMsg::decode(bytes)?;
                log::debug!("📨 普通消息处理");
            }
            ByteMessageType::GroupMessageType => {
                let _ = GroupMsg::decode(bytes)?;
                log::debug!("👥 群聊消息处理");
            }
            ByteMessageType::SystemNotificationType => {
                let _ = SystemNotificationMsg::decode(bytes)?;
                log::debug!("🔔 系统通知处理");
            }
            ByteMessageType::AckType => {
                let _ = AckMsg::decode(bytes)?;
                log::trace!("✅ ACK 收到");
            }
            ByteMessageType::HeartbeatType => {
                let _ = HeartbeatMsg::decode(bytes)?;
                last_heartbeat.store(now() as u64, Ordering::Relaxed);
                log::debug!("🫀 收到客户端心跳");
            }
            _ => {
                log::warn!("⚠️ 未知消息类型: {type_code}");
            }
        }
    }

    Ok(())
}
