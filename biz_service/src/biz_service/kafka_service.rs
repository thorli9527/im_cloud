use anyhow::{anyhow, Error};
use bytes::Bytes;
use common::config::KafkaConfig;
use once_cell::sync::OnceCell;
use prost::Message;
use rdkafka::admin::{AdminClient, AdminOptions, NewTopic, TopicReplication};
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::ClientConfig;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ByteMessageType {
    FriendMsg = 1,
    UserMessage = 2,
    GroupMessage = 3,
    Heartbeat = 4,
    SystemNotification = 5,
    AckMessage = 6,
}

impl ByteMessageType {
    pub fn from_u8(value: u8) -> Result<Self, Error> {
        match value {
            1 => Ok(ByteMessageType::FriendMsg),
            2 => Ok(ByteMessageType::UserMessage),
            3 => Ok(ByteMessageType::GroupMessage),
            4 => Ok(ByteMessageType::Heartbeat),
            5 => Ok(ByteMessageType::SystemNotification),
            6 => Ok(ByteMessageType::AckMessage),
            _ => Err(anyhow!("Unknown ByteMessageType: {}", value)),
        }
    }
}

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

        KafkaService { producer, config: cfg }
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
                        println!("🔁 Topic [{}] already exists, skipping", name);
                        continue;
                    }
                }
                Err((name, err)) => {
                    eprintln!("❌ Failed to create topic [{}]: {}", name, err);
                    std::process::exit(1);
                }
            }
        }
    }

    /// proto专有编码与解码
    pub async fn send_proto<M: Message>(&self, msg_type: &ByteMessageType, node_index:&u8, value: &M, key: &str, topic: &str) -> Result<(i32, i64), anyhow::Error> {
        let payload = value.encode_to_vec();
        let record = FutureRecord::to(topic).payload(&payload).key(key);
    
        let timeout = Duration::from_millis(50); // 可根据吞吐量和延迟优化
    
        match self.producer.send(record, timeout).await {
            Ok((partition, offset)) => {
                log::debug!("Kafka Protobuf 发送成功 => topic={}, partition={}, offset={}", topic, partition, offset);
                Ok((partition, offset))
            }
            Err((err, _)) => {
                log::error!("Kafka Protobuf 发送失败: {:?}", err);
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
