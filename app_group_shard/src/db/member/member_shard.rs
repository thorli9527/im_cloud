use arc_swap::ArcSwap;
use biz_service::protocol::common::GroupRoleType;
use dashmap::DashSet;
use std::sync::Arc;
use biz_service::protocol::rpc::arb_models::MemberRef;

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
    pub fn set_role(&self, id: &str, role: GroupRoleType) {
        if !self.id_set.contains(id) {
            return;
        }

        let current = self.index.load();
        let mut new_vec = Vec::with_capacity(current.len());
        let mut updated = false;

        for member in current.iter() {
            if member.id == id {
                let mut updated_member = (**member).clone();
                updated_member.role = role as i32;
                new_vec.push(Arc::new(updated_member));
                updated = true;
            } else {
                new_vec.push(member.clone());
            }
        }

        if updated {
            self.index.store(Arc::new(new_vec));
        }
    }
}
