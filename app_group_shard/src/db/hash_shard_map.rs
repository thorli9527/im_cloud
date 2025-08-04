use crate::db::member::member_list_wrapper::MemberListWrapper;
use crate::error::member_list_error::MemberListError;
use arc_swap::ArcSwap;
use biz_service::protocol::common::GroupRoleType;
use biz_service::protocol::rpc::arb_models::MemberRef;
use dashmap::{DashMap, DashSet};
use rand::{thread_rng, Rng};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use twox_hash::XxHash64;

#[derive(Debug)]
struct Shard {
    inner: ArcSwap<HashMap<Arc<str>, Arc<MemberListWrapper>>>, // key is Arc<str>
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
    shards: Arc<Vec<Shard>>,
    pub per_group_shard: usize,
    user_to_groups: DashMap<String, DashSet<Arc<str>>>, // user_id -> set of group keys
    group_key_intern: DashMap<String, Arc<str>>,        // intern pool
}

impl HashShardMap {
    pub fn new(shard_count: usize, per_group_shard: usize) -> Self {
        let shards = (0..shard_count).map(|_| Shard::default()).collect();
        Self {
            shards: Arc::new(shards),
            per_group_shard,
            user_to_groups: DashMap::new(),
            group_key_intern: DashMap::new(),
        }
    }

    /// Intern and dedupe group keys to shared Arc<str>
    fn intern_group_key(&self, key: &str) -> Arc<str> {
        if let Some(existing) = self.group_key_intern.get(key) {
            return existing.clone();
        }
        let arc: Arc<str> = Arc::from(key.to_string().into_boxed_str());
        match self.group_key_intern.entry(key.to_string()) {
            dashmap::mapref::entry::Entry::Occupied(o) => o.get().clone(),
            dashmap::mapref::entry::Entry::Vacant(v) => {
                v.insert(arc.clone());
                arc
            }
        }
    }

    fn get_shard_index(&self, key: &str) -> usize {
        let mut hasher = XxHash64::with_seed(0);
        key.hash(&mut hasher);
        (hasher.finish() as usize) % self.shards.len()
    }

    /// 获取或创建 wrapper，key 用 Arc<str> 共享
    fn get_or_create_wrapper(&self, key: &str) -> Arc<MemberListWrapper> {
        let group_arc = self.intern_group_key(key);
        let shard = &self.shards[self.get_shard_index(key)];
        loop {
            let current = shard.inner.load_full(); // Arc<HashMap<Arc<str>, ...>>
            if let Some(existing) = current.get(&group_arc) {
                return existing.clone();
            }
            let mut new_map = (*current).clone();
            let wrapper = Arc::new(MemberListWrapper::new_simple());
            new_map.insert(group_arc.clone(), wrapper.clone());
            let new_arc = Arc::new(new_map);
            let prev = shard.inner.compare_and_swap(&current, new_arc);
            if Arc::ptr_eq(&prev, &current) {
                return wrapper;
            }
            // else retry
        }
    }

    /// 通用重试：指数退避 + jitter（初始 10ms，上限 1s，最多 5 次）
    fn retry_op<T>(&self, mut op: impl FnMut() -> Result<T, MemberListError>) -> Result<T, MemberListError> {
        const MAX_RETRIES: usize = 5;
        let mut backoff = 10u64;
        const MAX_BACKOFF: u64 = 1000;

        for attempt in 0..MAX_RETRIES {
            match op() {
                Ok(res) => return Ok(res),
                Err(MemberListError::Retry) => {
                    if attempt + 1 == MAX_RETRIES {
                        break;
                    }
                    let mut rng = thread_rng();
                    let jitter: u64 = rng.gen_range(0..backoff);
                    thread::sleep(Duration::from_millis(backoff + jitter));
                    backoff = (backoff * 2).min(MAX_BACKOFF);
                }
                Err(e) => return Err(e),
            }
        }
        Err(MemberListError::Retry)
    }

    pub fn insert(&self, key: String, member: MemberRef) -> Result<(), MemberListError> {
        if key.is_empty() {
            return Ok(());
        }
        // use shared Arc<str> as group key
        let group_arc = self.intern_group_key(&key);
        let wrapper = self.get_or_create_wrapper(&key);
        self.retry_op(|| wrapper.add(member.clone()))?;
        self.user_to_groups.entry(member.id.clone()).or_insert_with(DashSet::new).insert(group_arc.clone());
        Ok(())
    }

    pub fn insert_many(&self, key: &str, members: Vec<MemberRef>) -> Result<(), MemberListError> {
        if key.is_empty() {
            return Ok(());
        }
        let group_arc = self.intern_group_key(key);
        let wrapper = self.get_or_create_wrapper(key);
        self.retry_op(|| wrapper.add_many(members.clone()))?;
        for m in members {
            self.user_to_groups.entry(m.id.clone()).or_insert_with(DashSet::new).insert(group_arc.clone());
        }
        Ok(())
    }

    pub fn get_page(&self, key: &str, page: usize, page_size: usize) -> Option<Vec<MemberRef>> {
        let group_arc = self.intern_group_key(key);
        let shard = &self.shards[self.get_shard_index(key)];
        shard.inner.load().get(&group_arc).map(|entry| entry.get_page(page, page_size))
    }

    pub fn get_online_ids(&self, key: &str) -> Vec<String> {
        let group_arc = self.intern_group_key(key);
        let shard = &self.shards[self.get_shard_index(key)];
        shard.inner.load().get(&group_arc).map(|entry| entry.get_online_ids()).unwrap_or_default()
    }

    pub fn get_online_page(&self, key: &str, page: usize, page_size: usize) -> Vec<String> {
        let group_arc = self.intern_group_key(key);
        let shard = &self.shards[self.get_shard_index(key)];
        shard.inner.load().get(&group_arc).map(|entry| entry.get_online_page(page, page_size)).unwrap_or_default()
    }

    pub fn get_online_count(&self, key: &str) -> usize {
        let group_arc = self.intern_group_key(key);
        let shard = &self.shards[self.get_shard_index(key)];
        shard.inner.load().get(&group_arc).map(|entry| entry.get_online_count()).unwrap_or(0)
    }

    pub fn remove(&self, key: &str, user_id: &str) -> Result<bool, MemberListError> {
        let group_arc = self.intern_group_key(key);
        let shard = &self.shards[self.get_shard_index(key)];
        if let Some(wrapper) = shard.inner.load().get(&group_arc) {
            let removed = self.retry_op(|| wrapper.remove(user_id))?;
            if removed {
                if let Some(mut set) = self.user_to_groups.get_mut(user_id) {
                    set.remove(&group_arc);
                    if set.is_empty() {
                        self.user_to_groups.remove(user_id);
                    }
                }
            }
            return Ok(removed);
        }
        Ok(false)
    }

    pub fn set_online(&self, key: &str, user_id: &str, online: bool) -> Result<(), MemberListError> {
        let group_arc = self.intern_group_key(key);
        let shard = &self.shards[self.get_shard_index(key)];
        if let Some(entry) = shard.inner.load().get(&group_arc) {
            return self.retry_op(|| entry.set_online(user_id, online));
        }
        Ok(())
    }

    pub fn change_role(&self, key: &str, user_id: &str, role: GroupRoleType) -> Result<(), MemberListError> {
        let group_arc = self.intern_group_key(key);
        let shard = &self.shards[self.get_shard_index(key)];
        if let Some(entry) = shard.inner.load().get(&group_arc) {
            return self.retry_op(|| entry.change_role(user_id, role));
        }
        Ok(())
    }

    /// 所有 group keys（shared Arc<str>，避免重复 alloc）
    pub fn all_keys(&self) -> Vec<Arc<str>> {
        self.shards.iter().flat_map(|shard| shard.inner.load().keys().cloned().collect::<Vec<_>>()).collect()
    }

    pub fn get_member_by_key(&self, key: &str) -> Vec<MemberRef> {
        let group_arc = self.intern_group_key(key);
        let shard = &self.shards[self.get_shard_index(key)];
        shard.inner.load().get(&group_arc).map(|entry| entry.get_all()).unwrap_or_default()
    }

    pub fn get_member_count_by_key(&self, key: &str) -> usize {
        let group_arc = self.intern_group_key(key);
        let shard = &self.shards[self.get_shard_index(key)];
        shard.inner.load().get(&group_arc).map(|entry| entry.len()).unwrap_or(0)
    }

    /// 清掉某个 group（包括反向索引更新）
    pub fn clear(&self, key: &str) {
        let group_arc = self.intern_group_key(key);
        let shard = &self.shards[self.get_shard_index(key)];

        if let Some(wrapper) = shard.inner.load().get(&group_arc) {
            let user_ids: Vec<String> = wrapper.get_all().into_iter().map(|m| m.id.clone()).collect();
            for uid in user_ids {
                if let Some(mut set) = self.user_to_groups.get_mut(&uid) {
                    set.remove(&group_arc);
                    if set.is_empty() {
                        self.user_to_groups.remove(&uid);
                    }
                }
            }
        }

        loop {
            let current = shard.inner.load_full();
            if !current.contains_key(&group_arc) {
                return;
            }
            let mut new_map = (*current).clone();
            new_map.remove(&group_arc);
            let new_arc = Arc::new(new_map);
            let prev = shard.inner.compare_and_swap(&current, new_arc);
            if Arc::ptr_eq(&prev, &current) {
                return;
            }
        }
    }

    /// 高效查某 user 属于哪些 groups（不排序）
    pub fn groups_for_user(&self, user_id: &str) -> Vec<Arc<str>> {
        if let Some(set) = self.user_to_groups.get(user_id) { set.iter().map(|r| r.clone()).collect() } else { Vec::new() }
    }

    /// 高效查某 user 的 group，返回排序后版本（稳定顺序）
    pub fn groups_for_user_sorted(&self, user_id: &str) -> Vec<Arc<str>> {
        let mut v = self.groups_for_user(user_id);
        v.sort();
        v
    }

    /// 重建反向索引（drift recovery）
    pub fn rebuild_user_index(&self) {
        self.user_to_groups.clear();
        for shard in self.shards.iter() {
            for (group, wrapper) in shard.inner.load().iter() {
                let members = wrapper.get_all();
                for m in members {
                    self.user_to_groups.entry(m.id.clone()).or_insert_with(DashSet::new).insert(group.clone());
                }
            }
        }
    }
}
