use biz_service::biz_service::kafka_socket_service::{KafkaInstanceService, TopicInfo};
use biz_service::protocol::rpc::arb_models::NodeType;
use biz_service::util::node_util::NodeUtil;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct KafkaService {
    pub kafka_list: Arc<Mutex<Vec<KafkaInstanceService>>>,
}

impl KafkaService {
    pub async fn new() -> Self {
        Self {
            kafka_list: Arc::new(Mutex::new(vec![])),
        }
    }
    pub async fn init() -> anyhow::Result<()> {
        let current = Self::new().await;
        current.rebuild().await;
        INSTANCE.set(Arc::new(current)).unwrap();
        Ok(())
    }
    pub async fn rebuild(&self) -> anyhow::Result<()> {
        let mut topic_list = Vec::new();
        topic_list.push(TopicInfo {
            topic_name: "".to_string(),
            partition_num: 0,
            replication_factor: 0,
        });
        let node_util = NodeUtil::get().await;
        let kafka_list = node_util.get_list(NodeType::GroupNode);
        for kafka in kafka_list {
            let brocker = kafka.kafka_addr.unwrap();
            let kafka_group_service = KafkaInstanceService::new(&brocker, &topic_list).await?;
            self.kafka_list.lock().await.push(kafka_group_service);
        }
        Ok(())
    }
    pub fn get() -> Arc<Self> {
        INSTANCE.get().expect("INSTANCE is not initialized").clone()
    }
}
//单例
static INSTANCE: once_cell::sync::OnceCell<Arc<KafkaService>> = once_cell::sync::OnceCell::new();
