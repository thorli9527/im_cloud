mod ack_handler;
mod message_handler;
mod group_member_change_handler;
mod auth_request_handler;
mod group_attribute_change_handler;
mod typing_notice_handler;

use crate::manager::socket_manager::{get_socket_manager, ConnectionId, ConnectionInfo, ConnectionMeta};
use crate::pb::protocol::envelope::Payload;
use crate::pb::protocol::envelope::Payload::AuthRequest;
use crate::pb::protocol::message_content::Content;
use crate::pb::protocol::Envelope;
use common::util::common_utils::build_uuid;
use common::util::date_util::now;
use futures::{SinkExt, StreamExt};
use prost::bytes::Bytes;
use prost::Message;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

/// 处理每个客户端连接
pub async fn handle_connection(
    stream: TcpStream,
) -> anyhow::Result<()> {
    let conn_id = build_uuid();
    let (read_half, write_half) = stream.into_split();
    let mut reader = FramedRead::new(read_half, LengthDelimitedCodec::new());
    let mut writer = FramedWrite::new(write_half, LengthDelimitedCodec::new());

    let (tx, mut rx) = mpsc::unbounded_channel::<Bytes>();
    let connection = ConnectionInfo {
        meta: ConnectionMeta {
            user_id: None,
            client_id: None,
            device_type: None,
        },
        sender: tx.clone(),
        last_heartbeat: Arc::new(AtomicU64::new(now() as u64)),
    };

    let manager = get_socket_manager();
    let conn = ConnectionId(conn_id.clone());
    manager.insert(conn.clone(), connection);

    // 启动写任务
    let write_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if writer.send(msg).await.is_err() {
                break;
            }
        }
    });

    // 主循环读取消息
    while let Some(frame) = reader.next().await {
        let bytes = frame?;
        match Envelope::decode(bytes.clone()) {
            Ok(envelope) => {
                match envelope.payload {
                    Some(AuthRequest(ack)) => {
                        println!(
                            "✅ 收到 Ack: msg_id={}, receiver={}",
                            ack.token, ack.client_id
                        );
                    }
                    Some(Payload::Ack(ack)) => {
                        println!(
                            "✅ 收到 Ack: msg_id={}, receiver={}",
                            ack.message_id, ack.ack_timestamp
                        );
                    }
                    Some(Payload::Message(msg)) => {
                        for item in msg.contents {
                            let content = item.content.unwrap();
                            match content {
                                Content::Text(text) => {
                                    println!("📨 Message from {}: {}", msg.sender_id, text.text);
                                }
                                Content::Image(au) => {}
                                Content::Audio(au) => {}
                                Content::Video(au) => {}
                                Content::Location(au) => {}
                                Content::File(au) => {}
                                Content::AvCall(au) => {}
                                Content::Custom(au) => {}
                                Content::Emoji(au) => {}
                                Content::Revoke(au) => {}
                                Content::Forward(au) => {}
                                Content::Quote(au) => {},
                            }
                        }
                    }
                    Some(Payload::ReadReceipt(read)) => {}
                    Some(Payload::GroupMemberChange(read)) => {}
                    Some(Payload::GroupAttributeChange(read)) => {}
                    Some(Payload::SystemNotification(read)) => {}
                    Some(Payload::TypingNotice(read)) => {}
                    Some(Payload::Heartbeat(read)) => {
                    }
                    _ => {
                        eprintln!("⚠️ 空消息？Envelope.kind 为 None");
                    }
                }
            }
            Err(e) => {
                eprintln!("❌ 解码 Envelope 失败: {:?}", e);
            }
        }
    }

    manager.remove(&conn);
    write_task.abort();
    Ok(())
}