use arc_swap::ArcSwap;
use biz_service::protocol::arb::rpc_arb_models::MemberRef;
use dashmap::DashSet;
use std::sync::Arc;
pub const SIMPLE_LIST_THRESHOLD: usize = 10_000;
#[derive(Debug, Default)]
pub struct MemberShard {
    index: ArcSwap<Vec<Arc<MemberRef>>>,
    id_set: DashSet<String>,
}

impl MemberShard {
    pub fn new() -> Self {
        Self {
            index: ArcSwap::from_pointee(Vec::new()),
            id_set: DashSet::new(),
        }
    }

    pub fn add_member(&self, member: Arc<MemberRef>) -> bool {
        if self.id_set.insert(member.id.clone()) {
            let mut new_vec = self.index.load().as_ref().clone();
            new_vec.push(member);
            self.index.store(Arc::new(new_vec));
            true
        } else {
            false
        }
    }

    pub fn remove_member(&self, id: &str) -> bool {
        if self.id_set.remove(id).is_some() {
            let new_vec: Vec<_> = self.index.load().iter().filter(|m| m.id != id).cloned().collect();
            self.index.store(Arc::new(new_vec));
            true
        } else {
            false
        }
    }

    pub fn contains(&self, id: &str) -> bool {
        self.id_set.contains(id)
    }

    pub fn get_all(&self) -> Vec<Arc<MemberRef>> {
        self.index.load().as_ref().clone()
    }

    pub fn len(&self) -> usize {
        self.id_set.len()
    }

    pub fn clear(&self) {
        self.id_set.clear();
        self.index.store(Arc::new(Vec::new()));
    }
}
