use biz_service::protocol::arb::rpc_arb_models::MemberRef;
use dashmap::{DashMap, DashSet};
use std::sync::Mutex;

#[derive(Debug, Default)]
pub struct SimpleMemberList {
    pub index: Mutex<Vec<MemberRef>>,
    pub id_set: DashSet<String>,
    pub online_map: DashMap<String, bool>,
}

impl Clone for SimpleMemberList {
    fn clone(&self) -> Self {
        let index = self.index.lock().unwrap().clone();
        let mut new_set = DashSet::new();
        for id in self.id_set.iter() {
            new_set.insert(id.clone());
        }
        let mut new_online = DashMap::new();
        for entry in self.online_map.iter() {
            new_online.insert(entry.key().clone(), *entry.value());
        }
        Self {
            index: Mutex::new(index),
            id_set: new_set,
            online_map: new_online,
        }
    }
}

impl SimpleMemberList {
    pub fn add(&self, item: MemberRef) {
        if item.id.is_empty() {
            return;
        }
        if self.id_set.insert(item.id.clone()) {
            self.index.lock().unwrap().push(item.clone());
        }
        self.online_map.insert(item.id, false);
    }

    pub fn add_many(&self, items: Vec<MemberRef>) {
        let mut index = self.index.lock().unwrap();
        for item in items {
            if !item.id.is_empty() && self.id_set.insert(item.id.clone()) {
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

    pub fn get_page(&self, page: usize, page_size: usize) -> Vec<MemberRef> {
        let index = self.index.lock().unwrap();
        index.iter().skip(page * page_size).take(page_size).cloned().collect()
    }

    pub fn get_all(&self) -> Vec<MemberRef> {
        self.index.lock().unwrap().clone()
    }

    pub fn len(&self) -> usize {
        self.id_set.len()
    }

    pub fn clear(&self) {
        self.online_map.clear();
        self.id_set.clear();
        self.index.lock().unwrap().clear();
    }
    pub fn is_online(&self, id: &str) -> bool {
        self.online_map.get(id).map(|v| *v).unwrap_or(false)
    }
    pub fn set_online(&self, id: &str, online: bool) {
        self.online_map.insert(id.to_string(), online);
    }
}
