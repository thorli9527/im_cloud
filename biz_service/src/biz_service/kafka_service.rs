use std::sync::Arc;
use std::time::Duration;
use mongodb::Database;
use once_cell::sync::OnceCell;
use rdkafka::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use serde::Serialize;
use common::config::KafkaConfig;
use crate::biz_service::agent_service::AgentService;
use std::fmt;
#[derive(Clone)]
pub struct KafkaService {
    producer: FutureProducer,
    config: KafkaConfig,
}
impl fmt::Debug for KafkaService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KafkaService")
            .field("config", &self.config)
            .finish()
    }
}
impl KafkaService {
    pub fn new(cfg: KafkaConfig) -> Self {
        let producer = ClientConfig::new()
            .set("bootstrap.servers", &cfg.brokers)
            .create()
            .expect("Kafka producer init failed");

        KafkaService { producer, config: cfg }
    }

    pub async fn send<T: Serialize>(
        &self,
        value: &T,
        key: &str,
        topic: &str,
    ) -> Result<(i32, i64), anyhow::Error> {
        let payload = serde_json::to_string(value)?;
        let record = FutureRecord::to(topic).payload(&payload).key(key);

        match self.producer.send(record, Duration::from_secs(0)).await {
            Ok((partition, offset)) => {
                log::info!(
                "Kafka message sent successfully. topic={}, partition={}, offset={}",
                topic,
                partition,
                offset
            );
                Ok((partition, offset))
            }
            Err((err, _)) => {
                log::error!("Kafka send failed: {:?}", err);
                Err(anyhow::Error::new(err))
            }
        }
    }

    pub fn init(cfg: KafkaConfig) {
        let instance = Self::new(cfg);
        SERVICE.set(Arc::new(instance)).expect("INSTANCE already initialized");
    }

    /// 获取单例
    pub fn get() -> Arc<Self> {
        SERVICE.get().expect("INSTANCE is not initialized").clone()
    }

}

static SERVICE: OnceCell<Arc<KafkaService>> = OnceCell::new();