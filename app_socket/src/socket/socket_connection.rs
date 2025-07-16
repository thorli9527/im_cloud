use crate::manager::socket_manager::{
    get_socket_manager, ConnectionId, ConnectionInfo, ConnectionMeta,
};
use crate::socket::heartbeat_handler::start_global_heartbeat_checker;
use crate::socket::login_handler::handle_login;
use crate::socket::logout_handler::handle_logout;
use anyhow::{anyhow, Result};
use biz_service::protocol::common::ByteMessageType;
use biz_service::protocol::msg::auth::{DeviceType, LoginReqMsg, LogoutReqMsg, OfflineStatueMsg, OnlineStatusMsg, SendVerificationCodeReqMsg};
use biz_service::protocol::msg::entity::{GroupMsgEntity, UserMsgEntity};
use biz_service::protocol::msg::friend::FriendEventMsg;
use biz_service::protocol::msg::group::{GroupCreateMsg, GroupDismissMsg};
use biz_service::protocol::msg::status::HeartbeatMsg;
use biz_service::protocol::msg::system::SystemNotificationMsg;
use biz_service::protocol::msg::user::UserFlushMsg;
use bytes::Buf;
use common::util::common_utils::build_uuid;
use common::util::date_util::now;
use futures::{SinkExt, StreamExt};
use prost::Message;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
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
    conn_key: &ConnectionId,
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
            ByteMessageType::LoginReqMsgType => {
                log::info!("🛂 收到w登录请求");
                let login_req = LoginReqMsg::decode(bytes)?;
                let i = login_req.device_type as i32;
                let device_type = DeviceType::from_i32(i).unwrap();
                let message_id = login_req.message_id;
                handle_login(
                    &message_id,
                    &login_req.username,
                    &login_req.password,
                    &device_type,
                )
                .await?;

            }
            ByteMessageType::LogoutReqMsgType => {
                let logout_req = LogoutReqMsg::decode(bytes)?;
                log::info!("🛂 收到w登录请求");
                if let Some(conn) = get_socket_manager().get(conn_key) {
                    if let ( Some(uid), Some(device_type)) = (
                        &conn.meta.uid,
                        &conn.meta.device_type,
                    ) {
                        let message_id = logout_req.message_id;
                        handle_logout(&message_id,  uid,device_type).await?;
                    } else {
                        log::warn!("Logout 请求未携带完整连接信息: {:?}", conn_key);
                    }
                } else {
                    log::warn!("找不到连接: {:?}", conn_key);
                }
            }
            ByteMessageType::SendVerificationCodeReqMsgType => {
                let _ = SendVerificationCodeReqMsg::decode(bytes)?;
                log::debug!("📨 处理验证码发送请求");
            }
            ByteMessageType::SystemNotificationMsgType => {
                let _ = SystemNotificationMsg::decode(bytes)?;
                log::info!("📢 系统通知消息");
            }
            ByteMessageType::UserFlushMsgType => {
                let _ = UserFlushMsg::decode(bytes)?;
                log::debug!("🔄 用户信息刷新");
            }
            ByteMessageType::OnlineStatusMsgType => {
                let _ = OnlineStatusMsg::decode(bytes)?;
                log::debug!("🟢 用户上线");
            }
            ByteMessageType::OfflineStatusMsgType => {
                let _ = OfflineStatueMsg::decode(bytes)?;
                log::debug!("🔴 用户下线");
            }
            ByteMessageType::UserMsgType => {
                let _ = UserMsgEntity::decode(bytes)?;
                log::debug!("📨 普通消息处理");
            }
            ByteMessageType::GroupMsgType => {
                let _ = GroupMsgEntity::decode(bytes)?;
                log::debug!("👥 群聊消息处理");
            }
            ByteMessageType::FriendEventMsgType => {
                let _ = FriendEventMsg::decode(bytes)?;
                log::debug!("👥 好友事件处理");
            }
            ByteMessageType::GroupCreateMsgType => {
                let _ = GroupCreateMsg::decode(bytes)?;
                log::info!("👥 创建群组事件处理");
            }
            ByteMessageType::GroupDismissMsgType => {
                let _ = GroupDismissMsg::decode(bytes)?;
                log::info!("👋 群组解散事件处理");
            }
            ByteMessageType::HeartbeatMsgType => {
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


