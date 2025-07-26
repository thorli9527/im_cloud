use biz_service::protocol::arb::rpc_arb_models::MemberRef;
use dashmap::{DashMap, DashSet};
use std::sync::Mutex;

pub const SIMPLE_LIST_THRESHOLD: usize = 10_000;

#[derive(Debug, Default)]
pub struct MemberShard {
    index: Mutex<Vec<MemberRef>>,
    id_set: DashSet<String>,
    online_map: DashMap<String, bool>,
}

impl MemberShard {
    pub fn new() -> Self {
        Self {
            index: Mutex::new(Vec::new()),
            id_set: DashSet::new(),
            online_map: DashMap::new(),
        }
    }

    pub fn add(&self, item: MemberRef) {
        if self.id_set.insert(item.id.clone()) {
            self.index.lock().unwrap().push(item.clone());
        }
        self.online_map.insert(item.id, false);
    }

    pub fn add_many(&self, items: Vec<MemberRef>) {
        let mut index = self.index.lock().unwrap();
        for item in items {
            if self.id_set.insert(item.id.clone()) {
                index.push(item.clone());
            }
            self.online_map.insert(item.id, false);
        }
    }

    pub fn remove(&self, id: &str) -> bool {
        self.online_map.remove(id);
        if self.id_set.remove(id).is_some() {
            let mut index = self.index.lock().unwrap();
            if let Some(pos) = index.iter().position(|v| v.id == id) {
                index.remove(pos);
            }
            true
        } else {
            false
        }
    }

    pub fn get_all(&self) -> Vec<MemberRef> {
        self.index.lock().unwrap().clone()
    }

    pub fn clear(&self) {
        self.id_set.clear();
        self.online_map.clear();
        self.index.lock().unwrap().clear();
    }

    pub fn len(&self) -> usize {
        self.id_set.len()
    }

    pub fn is_empty(&self) -> bool {
        self.id_set.is_empty()
    }
    pub fn is_online(&self, id: &str) -> bool {
        self.online_map.get(id).map(|v| *v).unwrap_or(false)
    }

    pub fn set_online(&self, id: &str, online: bool) {
        self.online_map.insert(id.to_string(), online);
    }
}
