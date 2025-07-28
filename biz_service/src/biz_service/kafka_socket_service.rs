use crate::protocol::common::ByteMessageType;
use anyhow::{anyhow, Result};
use common::config::KafkaConfig;
use once_cell::sync::OnceCell;
use prost::Message;
use rdkafka::admin::{AdminClient, AdminOptions, NewTopic, TopicReplication};
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::ClientConfig;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub struct KafkaService {
    producer: FutureProducer,
    config: KafkaConfig,
}
impl fmt::Debug for KafkaService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KafkaService").field("config", &self.config).finish()
    }
}
impl KafkaService {
    pub fn new(cfg: KafkaConfig) -> Self {
        let producer = ClientConfig::new().set("bootstrap.servers", &cfg.brokers).create().expect("Kafka producer init failed");

        KafkaService {
            producer,
            config: cfg,
        }
    }

    /// 初始化：创建 topics，然后注册单例
    pub async fn init(cfg: &KafkaConfig) {
        Self::create_topics_or_exit(&cfg.brokers, &[(&cfg.topic_single, 3, 1), (&cfg.topic_group, 2, 1)]).await;

        let instance = Self::new(cfg.clone());
        SERVICE.set(Arc::new(instance)).expect("KafkaService already initialized");
    }
    /// 异步创建多个 topic，如果已存在则退出程序
    pub async fn create_topics_or_exit(brokers: &str, topics: &[(&str, i32, i32)]) {
        let admin: AdminClient<_> = ClientConfig::new().set("bootstrap.servers", brokers).create().expect("Failed to create Kafka AdminClient");

        let topic_defs: Vec<_> = topics.iter().map(|(name, part, rep)| NewTopic::new(*name, *part, TopicReplication::Fixed(*rep))).collect();

        let results = admin.create_topics(&topic_defs, &AdminOptions::new()).await.expect("Kafka topic creation failed");

        for result in results {
            match result {
                Ok(name) => println!("✅ Created topic: {}", name),
                Err((name, err)) if err.to_string().contains("TopicAlreadyExists") => {
                    if err.to_string().contains("TopicAlreadyExists") {
                        continue;
                    }
                }
                Err((name, err)) => {
                    std::process::exit(1);
                }
            }
        }
    }

    /// 带类型标识的 Protobuf 消息发送（首字节 + Protobuf）
    pub async fn send_proto<M: Message>(&self, msg_type: &ByteMessageType, message: &M, message_id: &u64, topic: &str) -> Result<()> {
        let mut payload = Vec::with_capacity(1 + message.encoded_len());
        let message_id_str = &message_id.to_string();
        // 1️⃣ 插入类型码为首字节
        payload.push(*msg_type as u8);
        // 2️⃣ 编码 Protobuf 数据到后续部分
        message.encode(&mut payload)?;
        // 3️⃣ 构造 Kafka Record
        let record = FutureRecord::to(topic).payload(&payload).key(message_id_str);
        let timeout = Duration::from_millis(50);

        match self.producer.send(record, timeout).await {
            Ok(delivery) => {
                log::info!("✅ Kafka message sent to partition: {}, offset: {}", delivery.partition, delivery.offset);
                Ok(())
            }

            Err((err, _)) => {
                // todo 把错误写到 数据库 用job发送
                log::error!("❌ Kafka Protobuf 发送失败: {:?}", err);
                Err(anyhow!(err))
            }
        }
    }

    /// 获取单例
    pub fn get() -> Arc<Self> {
        SERVICE.get().expect("KafkaService is not initialized").clone()
    }
}

static SERVICE: OnceCell<Arc<KafkaService>> = OnceCell::new();
