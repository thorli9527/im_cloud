use dashmap::{DashMap, DashSet};
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use twox_hash::XxHash64;

use crate::db::member::member_shard::MemberShard;
use biz_service::protocol::common::GroupRoleType;
use biz_service::protocol::rpc::arb_models::MemberRef;

#[derive(Debug, Clone)]
pub struct ShardedMemberList {
    pub shard_count: usize,
    pub shards: DashMap<u64, Arc<MemberShard>>, // shard_id -> shard
    pub online_map: DashSet<String>,            // 仅存储在线成员 ID
}

impl ShardedMemberList {
    pub fn new(shard_count: usize) -> Self {
        Self {
            shards: DashMap::new(),
            shard_count,
            online_map: DashSet::new(),
        }
    }

    fn hash_id(&self, id: &str) -> u64 {
        let mut hasher = XxHash64::with_seed(0);
        id.hash(&mut hasher);
        hasher.finish() % self.shard_count as u64
    }

    fn get_or_create_shard(&self, id: &str) -> Arc<MemberShard> {
        let idx = self.hash_id(id);
        self.shards.entry(idx).or_insert_with(|| Arc::new(MemberShard::new())).clone()
    }

    pub fn add(&self, item: MemberRef) {
        let member = Arc::new(item);
        self.get_or_create_shard(&member.id).add_member(member);
        // 不自动上线
    }

    pub fn add_many(&self, items: Vec<MemberRef>) {
        let mut grouped = DashMap::new();
        for item in items {
            let member = Arc::new(item);
            let idx = self.hash_id(&member.id);
            grouped.entry(idx).or_insert_with(Vec::new).push(member);
        }
        for entry in grouped.into_iter() {
            let (idx, members) = entry;
            let shard = self.shards.entry(idx).or_insert_with(|| Arc::new(MemberShard::new()));
            for member in members {
                shard.add_member(member);
            }
        }
        // 不自动上线
    }

    pub fn remove(&self, id: &str) -> bool {
        self.online_map.remove(id); // 移除在线状态
        self.get_or_create_shard(id).remove_member(id)
    }

    pub fn get_page(&self, page: usize, page_size: usize) -> Vec<MemberRef> {
        let mut all: Vec<_> = self.shards.iter().flat_map(|shard| shard.get_all().into_iter()).map(|m| m.as_ref().clone()).collect();
        all.sort_by(|a, b| a.id.cmp(&b.id));
        all.into_iter().skip(page * page_size).take(page_size).collect()
    }

    pub fn get_online_page(&self, page: usize, page_size: usize) -> Vec<String> {
        let mut ids: Vec<String> = self.online_map.iter().map(|id| id.clone()).collect();
        ids.sort();
        ids.into_iter().skip(page * page_size).take(page_size).collect()
    }

    pub fn get_online_all(&self) -> Vec<String> {
        self.online_map.iter().map(|id| id.clone()).collect()
    }

    pub fn set_online(&self, id: &str) {
        self.online_map.insert(id.to_string());
    }

    pub fn set_offline(&self, id: &str) {
        self.online_map.remove(id);
    }

    pub fn is_online(&self, id: &str) -> bool {
        self.online_map.contains(id)
    }

    pub fn total_len(&self) -> usize {
        self.shards.iter().map(|s| s.len()).sum()
    }

    pub fn online_count(&self) -> usize {
        self.online_map.len()
    }

    pub fn clear(&self) {
        for shard in self.shards.iter() {
            shard.clear();
        }
        self.online_map.clear();
    }
    pub fn get_all(&self) -> Vec<MemberRef> {
        self.shards.iter().flat_map(|entry| entry.value().get_all()).map(|arc| arc.as_ref().clone()).collect()
    }

    pub fn set_role(&self, id: &str, role: GroupRoleType) {
        let shard = self.get_or_create_shard(id);
        shard.set_role(id, role);
    }
}
