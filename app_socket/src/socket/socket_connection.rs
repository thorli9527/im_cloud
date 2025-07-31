use crate::socket::handlers::auth::login_handler::handle_login;
use crate::socket::handlers::auth::logout_handler::handle_logout;
use crate::socket::handlers::heartbeat_handler::start_global_heartbeat_checker;
use crate::socket::socket_manager::{get_socket_manager, ConnectionId, ConnectionInfo, ConnectionMeta, SocketManager};
use anyhow::{anyhow, Result};
use biz_service::entitys::group_msg_entity::GroupMsgEntity;
use biz_service::entitys::user_msg_entity::UserMsgEntity;
use biz_service::protocol::common::ByteMessageType;
use biz_service::protocol::msg::auth::{DeviceType, LoginReqMsg, LogoutReqMsg, OfflineStatueMsg, OnlineStatusMsg, SendVerificationCodeReqMsg};
use biz_service::protocol::msg::friend::FriendEventMsg;
use biz_service::protocol::msg::group::{CreateGroupMsg, DestroyGroupMsg};
use biz_service::protocol::msg::status::{AckMsg, HeartbeatMsg};
use biz_service::protocol::msg::system::SystemNotificationMsg;
use biz_service::protocol::msg::user::UserFlushMsg;
use bytes::Buf;
use common::errors::AppError;
use common::util::common_utils::build_uuid;
use common::util::date_util::now;
use futures::{SinkExt, StreamExt};
use prost::Message;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

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
            uid: None,
            device_type: None,
        },
        sender: tx.clone(),
        last_heartbeat: last_heartbeat.clone(),
    };

    let manager = get_socket_manager();
    manager.insert(conn_key.clone(), connection);

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
    let result = read_loop(&mut reader, &conn_key, last_heartbeat.clone()).await;

    // 清理资源
    manager.remove(&conn_key);
    // 启动全局统一心跳检测任务
    start_global_heartbeat_checker();
    write_task.abort();
    result
}

/// 读取客户端数据 & 处理消息
async fn read_loop(
    reader: &mut FramedRead<tokio::net::tcp::OwnedReadHalf, LengthDelimitedCodec>,
    conn_id: &ConnectionId,
    last_heartbeat: Arc<AtomicU64>,
) -> Result<()> {
    while let Some(frame) = reader.next().await {
        let mut bytes = frame?;

        if bytes.len() < 1 {
            return Err(anyhow!("⚠️ 数据包长度不足"));
        }

        let type_code = bytes.get_u8();
        let message_type = ByteMessageType::try_from(type_code as i32).unwrap_or(ByteMessageType::UnknownByteMessageType);

        let socket_manager = SocketManager::get();
        match message_type {
            ByteMessageType::LoginReqMsgType => {
                log::info!("🛂 收到w登录请求");
                let login = LoginReqMsg::decode(bytes)?;
                let i = login.device_type as i32;
                let device_type = DeviceType::try_from(i)?;
                let message_id = login.message_id;
                if let Some(cached) = socket_manager.get_message_cache().get(&message_id) {
                    // 直接发送缓存的响应
                    log::info!("🔄 使用缓存响应，message_id: {}", message_id);
                    let (cache, _) = cached.value();
                    if let Err(e) = socket_manager.send_to_connection(conn_id, cache.clone()) {
                        log::error!("❌ 发送缓存响应失败：{}", "login");
                    }
                    return Ok(());
                }
                handle_login(conn_id, &message_id, &login.auth_type(), &login.auth_content, &login.password, &device_type).await;
            }
            ByteMessageType::LogoutReqMsgType => {
                let logout_req = LogoutReqMsg::decode(bytes)?;
                log::info!("🛂 收到w登录请求");
                if let Some(conn) = get_socket_manager().get_by_id(conn_id) {
                    if let (Some(uid), Some(device_type)) = (&conn.meta.uid, &conn.meta.device_type) {
                        let message_id = logout_req.message_id;
                        handle_logout(&message_id, uid, device_type).await?;
                    } else {
                        log::warn!("Logout 请求未携带完整连接信息: {:?}", conn_id);
                    }
                } else {
                    log::warn!("找不到连接: {:?}", conn_id);
                }
            }
            ByteMessageType::SendVerificationCodeReqMsgType => {
                let msg = SendVerificationCodeReqMsg::decode(bytes)?;
                log::debug!("📨 处理验证码发送请求");
            }
            // ByteMessageType::SystemNotificationMsgType => {
            //     let msg = SystemNotificationMsg::decode(bytes)?;
            //     log::info!("📢 系统通知消息");
            //     0 as u64
            // }
            ByteMessageType::UserFlushMsgType => {
                let msg = UserFlushMsg::decode(bytes)?;
                log::debug!("🔄 用户信息刷新");
            }
            ByteMessageType::OnlineStatusMsgType => {
                let msg = OnlineStatusMsg::decode(bytes)?;
                log::debug!("🟢 用户上线");
            }
            ByteMessageType::OfflineStatusMsgType => {
                let msg = OfflineStatueMsg::decode(bytes)?;
                log::debug!("🔴 用户下线");
            }
            ByteMessageType::UserMsgType => {
                let msg = UserMsgEntity::decode(bytes)?;
                log::debug!("📨 普通消息处理");
            }
            ByteMessageType::GroupMsgType => {
                let msg = GroupMsgEntity::decode(bytes)?;
                log::debug!("👥 群聊消息处理");
            }
            ByteMessageType::FriendEventMsgType => {
                let msg = FriendEventMsg::decode(bytes)?;
                log::debug!("👥 好友事件处理");
            }
            ByteMessageType::GroupCreateMsgType => {
                let msg = CreateGroupMsg::decode(bytes)?;
                log::info!("👥 创建群组事件处理");
            }
            ByteMessageType::GroupDismissMsgType => {
                let msg = DestroyGroupMsg::decode(bytes)?;
                log::info!("👋 群组解散事件处理");
            }
            ByteMessageType::HeartbeatMsgType => {
                let msg = HeartbeatMsg::decode(bytes)?;
                last_heartbeat.store(now() as u64, Ordering::Relaxed);
                log::debug!("🫀 收到客户端心跳");
            }
            ByteMessageType::AckMsgType => {
                let msg = AckMsg::decode(bytes)?;
                log::debug!("ACK 消息处理");
            }
            _ => {
                log::warn!("⚠️ 未知消息类型: {type_code}");
            }
        };
    }

    Ok(())
}
