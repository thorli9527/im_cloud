use crate::db::sharded_list::MemberShard;
use biz_service::protocol::arb::rpc_arb_models::MemberRef;
use dashmap::DashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use twox_hash::XxHash64;

#[derive(Debug, Clone)]
pub struct ShardedMemberList {
    pub shard_count: usize,
    pub shards: DashMap<u64, Arc<MemberShard>>,
    pub online_map: DashMap<String, bool>,
}

impl ShardedMemberList {
    pub fn new(shard_count: usize) -> Self {
        Self {
            shard_count,
            shards: DashMap::new(),
            online_map: DashMap::new(),
        }
    }

    fn hash_id(&self, id: &str) -> u64 {
        let mut hasher = XxHash64::with_seed(0);
        id.hash(&mut hasher);
        hasher.finish() % self.shard_count as u64
    }

    fn get_shard(&self, id: &str) -> Arc<MemberShard> {
        let idx = self.hash_id(id);
        self.shards.entry(idx).or_insert_with(|| Arc::new(MemberShard::new())).clone()
    }

    pub fn add(&self, item: MemberRef) {
        self.get_shard(&item.id).add(item);
    }

    pub fn add_many(&self, items: Vec<MemberRef>) {
        let mut map = DashMap::new();
        for item in items {
            let idx = self.hash_id(&item.id);
            map.entry(idx).or_insert_with(Vec::new).push(item);
        }
        for (idx, list) in map {
            self.shards.entry(idx).or_insert_with(|| Arc::new(MemberShard::new())).add_many(list);
        }
    }

    pub fn remove(&self, id: &str) -> bool {
        self.get_shard(id).remove(id)
    }

    pub fn get_page(&self, page: usize, page_size: usize) -> Vec<MemberRef> {
        let mut all = Vec::new();
        for shard in self.shards.iter() {
            all.extend(shard.get_all());
        }
        all.sort_by(|a, b| a.id.cmp(&b.id));
        all.into_iter().skip(page * page_size).take(page_size).collect()
    }

    pub fn total_len(&self) -> usize {
        self.shards.iter().map(|s| s.len()).sum()
    }

    pub fn clear(&self) {
        for shard in self.shards.iter() {
            shard.clear();
        }
    }
    pub fn set_online(&self, id: &str, online: bool) {
        self.online_map.insert(id.to_string(), online);
    }
}
