use crate::protocol::common::ByteMessageType;
use anyhow::{anyhow, Result};
use common::config::KafkaConfig;
use common::kafka::topic_info::TopicInfo;
use common::util::common_utils::build_md5;
use once_cell::sync::OnceCell;
use prost::Message;
use rdkafka::admin::{AdminClient, AdminOptions, NewTopic, TopicReplication};
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::ClientConfig;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub struct KafkaInstanceService {
    pub producer: Arc<FutureProducer>,
    pub broker_addr: String,
}
impl fmt::Debug for KafkaInstanceService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KafkaGroupService").field("producer", &"FutureProducer(...)").finish()
    }
}
impl KafkaInstanceService {
    pub fn broker_address(&self) -> Option<&str> {
        Some(&self.broker_addr) // 假设有 broker 字段
    }
    /// 主动关闭 Kafka producer，释放连接资源
    pub async fn shutdown(&self) {
        use rdkafka::producer::Producer;

        log::info!("Shutting down KafkaInstanceService for broker [{}]", self.broker_addr);

        // producer 是线程安全的，flush 是 sync 的，可在 tokio 阻塞中调用
        let producer = self.producer.clone();
        let broker = self.broker_addr.clone();

        tokio::task::spawn_blocking(move || {
            let timeout = std::time::Duration::from_secs(2);
            match producer.flush(timeout) {
                _ => {
                    log::info!("✅ Kafka producer flushed for broker [{}]", broker);
                }
            }
        })
        .await
        .unwrap_or_else(|e| {
            log::warn!("⚠️ Kafka shutdown task panicked: {:?}", e);
        });
    }
    pub async fn new(broker_addr: &str, topic_list: &Vec<TopicInfo>) -> Result<Self> {
        KafkaInstanceService::init(broker_addr, topic_list).await;
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", broker_addr)
            .set("security.protocol", "SASL_SSL")
            .set("sasl.mechanism", "PLAIN")
            .set("sasl.username", "admin")
            .set("sasl.password", build_md5(&broker_addr))
            // ✅ 性能相关配置
            .set("acks", "all")
            .set("enable.idempotence", "true")
            .set("queue.buffering.max.kbytes", "10240") // 默认4000，提升内存 buffer
            .set("queue.buffering.max.ms", "5") // 延迟聚合
            .set("compression.type", "lz4") // 压缩提升吞吐
            .set("batch.num.messages", "1000")
            .set("linger.ms", "5")
            .set("message.timeout.ms", "30000")
            .create()
            .map_err(|e| anyhow!("Kafka producer create failed for {broker_addr}: {e}"))?;

        Ok(Self {
            broker_addr: broker_addr.to_string(),
            producer: Arc::new(producer),
        })
    }

    /// 初始化所有 group 相关 topics（幂等）
    async fn init(brokers: &str, topic_list: &Vec<TopicInfo>) {
        let mut dynamic_topics = Vec::new();
        topic_list.iter().for_each(|topic| {
            dynamic_topics.push((topic.topic_name.clone(), topic.partitions, topic.replicas));
        });
        dynamic_topics.push(("group-node-msg".to_string(), 3, 1));
        if let Err(e) = Self::create_topics_or_exit(&brokers, &dynamic_topics).await {
            log::error!("❌ Kafka topic 创建失败: {e}");
        } else {
            log::info!("✅ KafkaGroupService 初始化完成，topic 数量 = {}", dynamic_topics.len());
        }
    }
    /// 异步创建多个 topic，如果已存在则退出程序
    pub async fn create_topics_or_exit(
        brokers: &str,
        topics: &Vec<(String, i32, i32)>,
    ) -> Result<()> {
        let admin: AdminClient<_> = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .create()
            .expect("Failed to create Kafka AdminClient");

        let topic_defs: Vec<_> = topics
            .iter()
            .map(|(name, part, rep)| NewTopic::new(name, *part, TopicReplication::Fixed(*rep)))
            .collect();

        let results = admin
            .create_topics(&topic_defs, &AdminOptions::new())
            .await
            .expect("Kafka topic creation failed");

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
        Ok(())
    }

    /// 带类型标识的 Protobuf 消息发送（首字节 + Protobuf）
    pub async fn send_proto<M: Message>(
        &self,
        msg_type: &ByteMessageType,
        message: &M,
        message_id: &u64,
        topic: &str,
    ) -> Result<()> {
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
                log::info!(
                    "✅ Kafka message sent to partition: {}, offset: {}",
                    delivery.partition,
                    delivery.offset
                );
                Ok(())
            }
            Err((err, _)) => {
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

static SERVICE: OnceCell<Arc<KafkaInstanceService>> = OnceCell::new();
