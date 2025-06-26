use crate::kafka::kafka_consumer::*;
use crate::manager::socket_manager::{get_socket_manager, ConnectionId, ConnectionInfo, ConnectionMeta};

use common::util::common_utils::build_uuid;
use common::util::date_util::now;

use futures::{SinkExt, StreamExt};
use prost::bytes::Bytes;
use prost::Message;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;

use biz_service::protocol::protocol::envelope::Payload;
use biz_service::protocol::protocol::envelope::Payload::AuthRequest;
use biz_service::protocol::protocol::message_content::Content;
use biz_service::protocol::protocol::Envelope;
use rdkafka::consumer::{CommitMode, Consumer};
use rdkafka::topic_partition_list::TopicPartitionList;
use rdkafka::Offset;
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
    while let Some(frame) = reader.next().await {
        let bytes = frame?;

        match Envelope::decode(bytes.clone()) {
            Ok(envelope) => match envelope.payload {
                Some(AuthRequest(auth)) => {
                    log::info!("🔐 收到 Auth 请求: token={}, client_id={}", auth.token, auth.uid);
                    // 可扩展调用 handler 模块
                }

                Some(Payload::Ack(ack)) => {
                    let msg_id = &ack.message_id;
                    let pending_acks = get_pending_acks();

                    if let Some((_, pending_meta)) = pending_acks.remove(msg_id) {
                        let consumer = get_consumer().expect("Kafka 消费者未初始化");
                        let mut tpl = TopicPartitionList::new();

                        // 构造正确的 Offset 提交对象
                        if tpl.add_partition_offset(
                            &pending_meta.topic,
                            pending_meta.partition,
                            Offset::Offset(pending_meta.offset + 1)
                        ).is_ok() {
                            if let Err(e) = consumer.commit(&tpl, CommitMode::Async) {
                                log::error!("❌ Kafka 提交失败 [{}]: {:?}", msg_id, e);
                            } else {
                                log::info!("✅ Kafka 消息提交成功 [{}]", msg_id);
                            }
                        } else {
                            log::warn!("⚠️ 构建 TopicPartitionList 失败 [{}]", msg_id);
                        }
                    } else {
                        log::warn!("⚠️ 找不到待确认的消息 [{}]", msg_id);
                    }
                }

                Some(Payload::Message(msg)) => {
                    for item in msg.contents {
                        if let Some(content) = item.content {
                            match content {
                                Content::Text(text) => {
                                    log::info!("📨 文本消息 [{}]: {}", msg.sender_id, text.text);
                                }
                                other => {
                                    log::debug!("📥 收到其他消息类型: {:?}", other);
                                }
                            }
                        }
                    }
                }

                Some(Payload::Heartbeat(_)) => {
                    // connection.last_heartbeat.store(now() as u64, Ordering::SeqCst);
                    log::debug!("💓 心跳更新: {:?}", conn_id);
                }

                Some(unknown) => {
                    log::debug!("⚠️ 未处理的消息类型: {:?}", unknown);
                }

                None => {
                    log::warn!("⚠️ 收到空 Payload 消息");
                }
            },

            Err(e) => {
                log::error!("❌ Envelope 解码失败: {:?}", e);
            }
        }
    }

    manager.remove(&conn_key);
    write_task.abort();
    Ok(())
}
