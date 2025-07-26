use crate::db::member::member_list_wrapper::MemberListWrapper;
use arc_swap::ArcSwap;
use biz_service::protocol::arb::rpc_arb_models::MemberRef;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use twox_hash::XxHash64;

#[derive(Debug)]
struct Shard {
    inner: ArcSwap<HashMap<String, Arc<MemberListWrapper>>>,
}

impl Default for Shard {
    fn default() -> Self {
        Self {
            inner: ArcSwap::from_pointee(HashMap::new()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct HashShardMap {
    shards: Arc<Vec<Arc<Shard>>>,
    per_group_shard: usize,
}

impl HashShardMap {
    pub fn new(shard_count: usize, per_group_shard: usize) -> Self {
        let shards = (0..shard_count).map(|_| Arc::new(Shard::default())).collect();
        Self {
            shards: Arc::new(shards),
            per_group_shard,
        }
    }

    fn get_shard_index(&self, key: &str) -> usize {
        let mut hasher = XxHash64::with_seed(0);
        key.hash(&mut hasher);
        (hasher.finish() as usize) % self.shards.len()
    }

    fn get_shard(&self, key: &str) -> Arc<Shard> {
        let idx = self.get_shard_index(key);
        self.shards[idx].clone()
    }

    pub fn insert(&self, key: String, member: MemberRef) {
        if key.is_empty() {
            return;
        }
        let shard = self.get_shard(&key);
        let mut new_map = (**shard.inner.load()).clone();
        let entry = new_map.entry(key.clone()).or_insert_with(|| Arc::new(MemberListWrapper::new_simple()));
        entry.add(member);
        if entry.should_upgrade() {
            entry.upgrade(self.per_group_shard);
        }
        shard.inner.store(Arc::new(new_map));
    }

    pub fn insert_many(&self, key: &str, members: Vec<MemberRef>) {
        if key.is_empty() {
            return;
        }
        let shard = self.get_shard(key);
        let mut new_map = (**shard.inner.load()).clone();
        let entry = new_map.entry(key.to_string()).or_insert_with(|| Arc::new(MemberListWrapper::new_simple()));
        entry.add_many(members);
        if entry.should_upgrade() {
            entry.upgrade(self.per_group_shard);
        }
        shard.inner.store(Arc::new(new_map));
    }

    pub fn get_page(&self, key: &str, page: usize, page_size: usize) -> Option<Vec<MemberRef>> {
        self.get_shard(key).inner.load().get(key).map(|entry| entry.get_page(page, page_size))
    }

    pub fn get_online_ids(&self, key: &str) -> Vec<String> {
        self.get_shard(key).inner.load().get(key).map(|entry| entry.get_online_ids()).unwrap_or_default()
    }

    pub fn get_online_page(&self, key: &str, page: usize, page_size: usize) -> Vec<String> {
        self.get_shard(key).inner.load().get(key).map(|entry| entry.get_online_page(page, page_size)).unwrap_or_default()
    }

    pub fn get_online_count(&self, key: &str) -> usize {
        self.get_shard(key).inner.load().get(key).map(|entry| entry.get_online_count()).unwrap_or(0)
    }

    pub fn remove(&self, key: &str, user_id: &str) -> bool {
        self.get_shard(key).inner.load().get(key).map(|entry| entry.remove(user_id)).unwrap_or(false)
    }

    pub fn set_online(&self, key: &str, user_id: &str, online: bool) {
        if let Some(entry) = self.get_shard(key).inner.load().get(key) {
            entry.set_online(user_id, online);
        }
    }

    pub fn clear(&self, key: &str) {
        let shard = self.get_shard(key);
        let mut new_map = (**shard.inner.load()).clone();
        new_map.remove(key);
        shard.inner.store(Arc::new(new_map));
    }

    pub fn all_keys(&self) -> Vec<String> {
        self.shards.iter().flat_map(|shard| shard.inner.load().keys().cloned().collect::<Vec<_>>()).collect()
    }
    pub fn get_member_by_key(&self, key: &str) -> Vec<MemberRef> {
        self.get_shard(key).inner.load().get(key).map(|entry| entry.get_all()).unwrap_or_default()
    }
    pub fn get_member_count_by_key(&self, key: &str) -> usize {
        self.get_shard(key).inner.load().get(key).map(|entry| entry.len()).unwrap_or(0)
    }
}
