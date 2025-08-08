use dashmap::DashMap;
use once_cell::sync::OnceCell;
use std::sync::Arc;
use crate::protocol::arb::arb_models::{NodeInfo, NodeType};

#[derive(Debug)]
pub struct NodeUtil {
    pub node_address_list: DashMap<NodeType, Vec<NodeInfo>>,
}
impl NodeUtil {
    fn new() -> Self {
        Self {
            node_address_list: DashMap::new(),
        }
    }
    pub async fn init() {
        if INSTANCE.get().is_some() {
            return;
        }
        let util = Self::new();
        INSTANCE.set(Arc::new(util)).expect("NodeAddressUtil error");
    }
    pub fn push_list(&self, node_type: NodeType, vec: Vec<NodeInfo>) {
        // 使用 entry API 安全操作 DashMap
        let mut entry = self.node_address_list.entry(node_type).or_insert_with(Vec::new);
        entry.clear();
        // 合并传入的 vec 到现有列表中
        entry.extend(vec);
    }
    pub fn reset_list(&self, node_type: NodeType, vec: Vec<NodeInfo>) {
        let mut entry = self.node_address_list.entry(node_type).or_insert_with(Vec::new);
    }
    pub fn remove(&self, node_type: NodeType, node: &NodeInfo) {
        self.node_address_list.entry(node_type).and_modify(|list| {
            list.retain(|item| item != node);
        });
    }
    pub async fn get() -> Arc<Self> {
        return INSTANCE.get().unwrap().clone();
    }
    pub fn get_list(&self, node_type: NodeType) -> Vec<NodeInfo> {
        return self.node_address_list.get(&node_type).unwrap().to_vec();
    }
}
static INSTANCE: OnceCell<Arc<NodeUtil>> = OnceCell::new();
