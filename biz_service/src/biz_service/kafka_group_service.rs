use crate::protocol::common::{ByteMessageType};
use anyhow::{Result, anyhow};
use common::config::KafkaConfig;
use once_cell::sync::OnceCell;
use prost::Message;
use rdkafka::ClientConfig;
use rdkafka::admin::{AdminClient, AdminOptions, NewTopic, TopicReplication};
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::statistics::Broker;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use strum::IntoEnumIterator;
use strum_macros::{AsRefStr, Display, EnumIter, EnumString};
use utoipa::openapi::security::Password;
use common::util::common_utils::build_md5;
use crate::manager::init;
use crate::protocol::msg::group_models::GroupNodeMsgType;

pub const GROUP_NODE_MSG_TOPIC : &str= "group-node-msg";
#[derive(Clone)]
pub struct KafkaGroupService {
    pub producer: Arc<FutureProducer>,
}
impl fmt::Debug for KafkaGroupService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KafkaGroupService")
            .field("producer", &"FutureProducer(...)")
            .finish()
    }
}
impl KafkaGroupService {
    pub async fn new(broker_addr: &str) -> Result<Self> {
        KafkaGroupService::init(broker_addr).await;
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", broker_addr)
            .set("security.protocol", "SASL_SSL")
            .set("sasl.mechanism", "PLAIN")
            .set("sasl.username", "admin")
            .set("sasl.password", build_md5(&broker_addr))
            // ✅ 性能相关配置
            .set("queue.buffering.max.kbytes", "10240")     // 默认4000，提升内存 buffer
            .set("queue.buffering.max.ms", "5")              // 延迟聚合
            .set("compression.type", "lz4")                  // 压缩提升吞吐
            .set("acks", "1")                                // 成本均衡（0/1/all）
            .set("batch.num.messages", "1000")
            .set("linger.ms", "5")
            .set("message.timeout.ms", "30000")
            .create()
            .map_err(|e| anyhow!("Kafka producer create failed for {broker_addr}: {e}"))?;

        Ok(Self {
            producer: Arc::new(producer),
        })
    }

    /// 发送 Protobuf 消息（带类型码首字节）
    pub async fn send_proto<M: Message>(
        &self,
        msg_type: &GroupNodeMsgType,
        message: &M,
        message_id: &str
    ) -> Result<()> {
        let mut payload = Vec::with_capacity(1 + message.encoded_len());
        payload.push(*msg_type as u8);
        message.encode(&mut payload)?;

        let key = message_id.to_string();

        let record = FutureRecord::to(GROUP_NODE_MSG_TOPIC)
            .payload(&payload)
            .key(&key);
        let timeout = Duration::from_millis(500); // ✅ 更合理的超时

        match self.producer.send(record, timeout).await {
            Ok(delivery) => {
                log::debug!(
                    "✅ Kafka[{}] sent msg_id={} → partition={} offset={}",
                    GROUP_NODE_MSG_TOPIC,
                    message_id,
                    delivery.partition,
                    delivery.offset
                );
                Ok(())
            }
            Err((err, _)) => {
                log::error!(
                    "❌ Kafka send failed. topic={} msg_id={} err={:?}",
                    GROUP_NODE_MSG_TOPIC,
                    message_id,
                    err
                );
                Err(anyhow!(err))
            }
        }
    }

    /// 初始化所有 group 相关 topics（幂等）
    async fn init(brokers: &str) {
        let mut dynamic_topics = Vec::new();
        dynamic_topics.push(("group-node-msg".to_string(), 3, 1));
        if let Err(e) = Self::create_topics(&brokers, &dynamic_topics).await {
            log::error!("❌ Kafka topic 创建失败: {e}");
        } else {
            log::info!("✅ KafkaGroupService 初始化完成，topic 数量 = {}", dynamic_topics.len());
        }
    }

    /// 创建 topics（失败日志提示，但不中断）
    pub async fn create_topics(
        brokers: &str,
        topics: &[(String, i32, i32)],
    ) -> Result<()> {
        let admin: AdminClient<_> = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("security.protocol", "SASL_SSL") // 或 "SASL_PLAINTEXT" 视你的集群配置而定
            .set("sasl.mechanism", "PLAIN")       // 也可以是 SCRAM-SHA-256 / SCRAM-SHA-512 等
            .set("sasl.username", "admin")
            .set("sasl.password", build_md5(brokers))
            .create()
            .map_err(|e| anyhow!("Failed to create Kafka AdminClient: {e}"))?;

        let topic_defs: Vec<_> = topics
            .iter()
            .map(|(name, part, rep)| NewTopic::new(name, *part, TopicReplication::Fixed(*rep)))
            .collect();

        let results = admin
            .create_topics(&topic_defs, &AdminOptions::new())
            .await
            .map_err(|e| anyhow!("Topic creation RPC failed: {e}"))?;

        for res in results {
            match res {
                Ok(name) => log::info!("✅ Topic created: {}", name),
                Err((name, err)) if err.to_string().contains("TopicAlreadyExists") => {
                    log::debug!("🔁 Topic [{}] already exists, skipping", name);
                }
                Err((name, err)) => {
                    log::warn!("⚠️ Topic [{}] creation failed: {}", name, err);
                }
            }
        }

        Ok(())
    }
}
