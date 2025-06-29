use crate::kafka::kafka_consumer::*;
use crate::manager::socket_manager::{get_socket_manager, ConnectionId, ConnectionInfo, ConnectionMeta};

use common::util::common_utils::build_uuid;
use common::util::date_util::now;

use anyhow::anyhow;
use anyhow::Result;
use biz_service::biz_service::kafka_service::ByteMessageType;
use biz_service::protocol;
use biz_service::protocol::entity::UserMessage;
use biz_service::protocol::friend::FriendEventMessage;
use biz_service::protocol::message::message_content::Content;
use biz_service::protocol::message::GroupEventMessage;
use biz_service::protocol::status::{AckMessage, Heartbeat};
use biz_service::protocol::system::SystemEventMessage;
use bytes::Buf;
use futures::{SinkExt, StreamExt};
use prost::bytes::Bytes;
use prost::Message;
use rdkafka::consumer::{CommitMode, Consumer};
use rdkafka::topic_partition_list::TopicPartitionList;
use rdkafka::Offset;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

/// 处理每个客户端连接
pub async fn handle_connection(stream: TcpStream) -> anyhow::Result<()> {
    let conn_id = build_uuid();
    let (read_half, write_half) = stream.into_split();
    let mut reader = FramedRead::new(read_half, LengthDelimitedCodec::new());
    let mut writer = FramedWrite::new(write_half, LengthDelimitedCodec::new());

    let (tx, mut rx) = mpsc::unbounded_channel::<Bytes>();
    let connection = ConnectionInfo { meta: ConnectionMeta { user_id: None, client_id: None, device_type: None }, sender: tx.clone(), last_heartbeat: Arc::new(AtomicU64::new(now() as u64)) };

    let manager = get_socket_manager();
    let conn_key = ConnectionId(conn_id.clone());
    manager.insert(conn_key.clone(), connection);

    // 写任务（发送数据到客户端）
    let write_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Err(e) = writer.send(msg).await {
                log::warn!("🔌 客户端写入失败: {:?}", e);
                break;
            }
        }
    });

    // 主循环读取客户端消息
    async fn read_loop(
        reader: &mut FramedRead<tokio::net::tcp::OwnedReadHalf, LengthDelimitedCodec>,
        conn_key: ConnectionId,
    ) -> Result<()> {
        while let Some(frame) = reader.next().await {
            let mut bytes = frame?;

            if bytes.len() < 1 {
                return Err(anyhow!("数据包长度不足"));
            }

            let type_code = bytes.get_u8(); // 取出类型字节
            let message_type = ByteMessageType::from_u8(type_code)?;

            match message_type {
                ByteMessageType::FriendMsg => {
                    let msg = FriendEventMessage::decode(bytes)?;
                    // handle_friend_message(conn_key.clone(), msg).await?;
                }
                ByteMessageType::UserMessage => {
                    let msg = UserMessage::decode(bytes)?;
                    // handle_user_message(conn_key.clone(), msg).await?;
                }
                ByteMessageType::GroupMessage => {
                    let msg = GroupEventMessage::decode(bytes)?;
                    // handle_group_message(conn_key.clone(), msg).await?;
                }
                ByteMessageType::Heartbeat => {
                    let msg = Heartbeat::decode(bytes)?;
                    // handle_group_message(conn_key.clone(), msg).await?;
                }               
                ByteMessageType::SystemNotification => {
                    let msg = SystemEventMessage::decode(bytes)?;
                    // handle_group_message(conn_key.clone(), msg).await?;
                }
                ByteMessageType::AckMessage => {
                    let msg = AckMessage::decode(bytes)?;
                    // handle_group_message(conn_key.clone(), msg).await?;
                }
            }
        }

        Ok(())
    }

    manager.remove(&conn_key);
    write_task.abort();
    Ok(())
}
