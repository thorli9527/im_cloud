use biz_service::kafka_util::kafka_producer::KafkaInstanceService;
use biz_service::kafka_util::node_util::NodeUtil;
use biz_service::protocol::rpc::arb_models::NodeType;
use common::kafka::topic_info::{MSG_SEND_TOPIC_INFO, ONLINE_TOPIC_INFO, USER_PRESENCE_TOPIC_INFO};
use log::warn;
use std::collections::{HashMap, HashSet};
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
        current.rebuild().await?;
        INSTANCE.set(Arc::new(current)).unwrap();
        Ok(())
    }

    /// 差异 rebuild：仅构建新增 broker 实例，复用未变，释放无效
    pub async fn rebuild(&self) -> anyhow::Result<()> {
        let topic_list = vec![
            ONLINE_TOPIC_INFO.clone(),
            MSG_SEND_TOPIC_INFO.clone(),
            USER_PRESENCE_TOPIC_INFO.clone(),
        ];

        let node_util = NodeUtil::get().await;

        // 获取所有现存节点（GroupNode + SocketNode）
        let mut all_nodes = node_util.get_list(NodeType::GroupNode);
        all_nodes.extend(node_util.get_list(NodeType::SocketNode));

        // 构建当前 broker 集合
        let new_brokers: HashSet<String> =
            all_nodes.iter().filter_map(|node| node.kafka_addr.clone()).collect();

        let mut lock = self.kafka_list.lock().await;

        // 构建 broker -> instance 映射（旧）
        let mut existing_map: HashMap<String, KafkaInstanceService> =
            lock.drain(..).map(|svc| (svc.broker_addr.clone(), svc)).collect();

        let mut updated_list = Vec::with_capacity(new_brokers.len());

        for broker in new_brokers {
            if let Some(existing_svc) = existing_map.remove(&broker) {
                // 已存在 → 复用
                updated_list.push(existing_svc);
            } else {
                // 新 broker → 创建
                match KafkaInstanceService::new(&broker, &topic_list).await {
                    Ok(new_svc) => updated_list.push(new_svc),
                    Err(err) => {
                        warn!(
                            "Failed to create KafkaInstanceService for broker [{}]: {}",
                            broker, err
                        );
                    }
                }
            }
        }

        // 不在 new_brokers 中的实例自动 drop（因为 existing_map 未被复用）
        *lock = updated_list;
        for (_, svc) in existing_map {
            svc.shutdown().await;
        }
        Ok(())
    }

    pub fn get() -> Arc<Self> {
        INSTANCE.get().expect("INSTANCE is not initialized").clone()
    }
}
//单例
static INSTANCE: once_cell::sync::OnceCell<Arc<KafkaService>> = once_cell::sync::OnceCell::new();
