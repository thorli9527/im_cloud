use crate::db::intern_pool::InternPool;
use crate::db::member::member_list_wrapper::MemberListWrapper;
use crate::error::member_list_error::MemberListError;
use arc_swap::ArcSwap;
use biz_service::protocol::common::GroupRoleType;
use dashmap::{DashMap, DashSet};
use rand::{rng, Rng};
use std::hash::Hasher;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use twox_hash::XxHash64;
use biz_service::protocol::arb::arb_models::MemberRef;

/// 无锁分片 map，用于管理 group -> MemberListWrapper
#[derive(Debug)]
struct Shard {
    inner: ArcSwap<DashMap<Arc<str>, Arc<MemberListWrapper>>>,
}

impl Default for Shard {
    fn default() -> Self {
        Shard {
            inner: ArcSwap::from_pointee(DashMap::new()),
        }
    }
}

/// HashShardMap: 按 key 哈希分片后管理 MemberListWrapper 和反向索引
#[derive(Debug, Clone)]
pub struct HashShardMap {
    /// 全局字符串 intern 池
    pool: Arc<InternPool>,
    /// 分片容器
    shards: Arc<Vec<Shard>>,
    /// 兼容字段：每 group 期望的 shard 数，可留存或移除
    pub per_group_shard: usize,
    /// 反向索引: user_id -> 所在 groups（Arc<str>）集合
    user_to_groups: DashMap<Arc<str>, DashSet<Arc<str>>>,
}

impl HashShardMap {
    /// 构造 HashShardMap
    pub fn new(shard_count: usize, per_group_shard: usize) -> Self {
        let shards = (0..shard_count).map(|_| Shard::default()).collect();
        let hot_capacity = (1_000_000 / shard_count.max(1)).max(1);
        Self {
            pool: Arc::new(InternPool::new(shard_count, hot_capacity)),
            shards: Arc::new(shards),
            per_group_shard,
            user_to_groups: DashMap::new(),
        }
    }

    #[inline]
    fn get_shard_index(&self, key: &str) -> usize {
        let mut hasher = XxHash64::with_seed(0);
        hasher.write(key.as_bytes());
        (hasher.finish() as usize) % self.shards.len()
    }

    /// 获取或创建 MemberListWrapper（CAS）
    fn get_or_create_wrapper(&self, group: &Arc<str>) -> Arc<MemberListWrapper> {
        let shard = &self.shards[self.get_shard_index(group)];
        loop {
            let current = shard.inner.load_full();
            if let Some(w) = current.get(group) {
                return w.clone();
            }
            let mut new_map = (*current).clone();
            let wrapper = Arc::new(MemberListWrapper::new_simple());
            new_map.insert(group.clone(), wrapper.clone());
            let new_arc = Arc::new(new_map);
            let prev = shard.inner.compare_and_swap(&current, new_arc);
            if Arc::ptr_eq(&prev, &current) {
                return wrapper;
            }
        }
    }

    /// 通用重试 + backoff
    fn retry_op<T>(&self, mut op: impl FnMut() -> Result<T, MemberListError>) -> Result<T, MemberListError> {
        const MAX_RETRIES: usize = 5;
        const INITIAL_MS: u64 = 100;
        const MAX_MS: u64 = 1000;
        let mut backoff = INITIAL_MS;
        for attempt in 0..MAX_RETRIES {
            match op() {
                Ok(v) => return Ok(v),
                Err(MemberListError::Retry) if attempt + 1 < MAX_RETRIES => {
                    let mut rng = rng();
                    let jitter = rng.random_range(0..backoff);
                    thread::sleep(Duration::from_millis(backoff + jitter));
                    backoff = (backoff * 2).min(MAX_MS);
                }
                Err(e) => return Err(e),
                _ => break,
            }
        }
        Err(MemberListError::Retry)
    }

    /// 插入单个成员
    pub fn insert(&self, key: String, member: MemberRef) -> Result<(), MemberListError> {
        if key.is_empty() {
            return Ok(());
        }
        let gkey = self.pool.intern(&key);
        let ukey = self.pool.intern(&member.id);
        let wrapper = self.get_or_create_wrapper(&gkey);
        self.retry_op(|| wrapper.add(member.clone()))?;
        self.user_to_groups.entry(ukey.clone()).or_insert_with(DashSet::new).insert(gkey);
        Ok(())
    }

    /// 批量插入
    pub fn insert_many(&self, key: &str, members: Vec<MemberRef>) -> Result<(), MemberListError> {
        if key.is_empty() {
            return Ok(());
        }
        let gkey = self.pool.intern(key);
        let wrapper = self.get_or_create_wrapper(&gkey);
        self.retry_op(|| wrapper.add_many(members.clone()))?;
        for m in members {
            let ukey = self.pool.intern(&m.id);
            self.user_to_groups.entry(ukey).or_insert_with(DashSet::new).insert(gkey.clone());
        }
        Ok(())
    }

    /// 获取分页成员
    pub fn get_page(&self, key: &str, page: usize, page_size: usize) -> Option<Vec<MemberRef>> {
        let gkey = self.pool.intern(key);
        let shard = &self.shards[self.get_shard_index(key)];
        shard.inner.load().get(&gkey).map(|w| w.get_page(page, page_size))
    }

    /// 获取在线ID
    pub fn get_online_ids(&self, key: &str) -> Vec<String> {
        let gkey = self.pool.intern(key);
        let shard = &self.shards[self.get_shard_index(key)];
        shard.inner.load().get(&gkey).map(|w| w.get_online_ids()).unwrap_or_default()
    }

    /// 删除成员
    pub fn remove(&self, key: &str, user_id: &str) -> Result<bool, MemberListError> {
        let gkey = self.pool.intern(key);
        let ukey = self.pool.intern(user_id);
        let shard = &self.shards[self.get_shard_index(key)];
        if let Some(wrapper) = shard.inner.load().get(&gkey) {
            let removed = self.retry_op(|| wrapper.remove(user_id))?;
            if removed {
                if let Some(mut set) = self.user_to_groups.get_mut(&ukey) {
                    set.remove(&gkey);
                    if set.is_empty() {
                        self.user_to_groups.remove(&ukey);
                    }
                }
            }
            Ok(removed)
        } else {
            Ok(false)
        }
    }

    /// 设置在线状态
    pub fn set_online(&self, key: &str, user_id: &str, online: bool) -> Result<(), MemberListError> {
        let gkey = self.pool.intern(key);
        let shard = &self.shards[self.get_shard_index(key)];
        if let Some(wrapper) = shard.inner.load().get(&gkey) {
            return self.retry_op(|| {
                wrapper.set_online(user_id, online);
                Ok(())
            });
        }
        Ok(())
    }

    /// 改变角色
    pub fn change_role(&self, key: &str, user_id: &str, role: GroupRoleType) -> Result<(), MemberListError> {
        let gkey = self.pool.intern(key);
        let shard = &self.shards[self.get_shard_index(key)];
        if let Some(wrapper) = shard.inner.load().get(&gkey) {
            return self.retry_op(|| {
                wrapper.change_role(user_id, role);
                Ok(())
            });
        }
        Ok(())
    }

    /// 清空 group
    pub fn clear(&self, key: &str) {
        let gkey = self.pool.intern(key);
        let shard = &self.shards[self.get_shard_index(key)];
        if let Some(wrapper) = shard.inner.load().get(&gkey) {
            for m in wrapper.get_all() {
                let ukey = self.pool.intern(&m.id);
                if let Some(mut set) = self.user_to_groups.get_mut(&ukey) {
                    set.remove(&gkey);
                    if set.is_empty() {
                        self.user_to_groups.remove(&ukey);
                    }
                }
            }
        }
        loop {
            let current = shard.inner.load_full();
            if !current.contains_key(&gkey) {
                break;
            }
            let mut new_map = (*current).clone();
            new_map.remove(&gkey);
            let new_arc = Arc::new(new_map);
            let prev = shard.inner.compare_and_swap(&current, new_arc);
            if Arc::ptr_eq(&prev, &current) {
                break;
            }
        }
    }

    /// 获取用户所在 group list
    pub fn user_group_list(&self, user_id: &str) -> Vec<Arc<str>> {
        let ukey = self.pool.intern(user_id);
        if let Some(set) = self.user_to_groups.get(&ukey) { set.iter().map(|r| r.key().clone()).collect() } else { Vec::new() }
    }

    /// 获取所有 group key
    pub fn all_keys(&self) -> Vec<Arc<str>> {
        let mut keys = Vec::new();
        for shard in self.shards.iter() {
            let map = shard.inner.load();
            for entry in map.iter() {
                keys.push(entry.key().clone());
            }
        }
        keys
    }

    /// 根据 group key 获取所有成员
    pub fn get_member_by_key(&self, key: &str) -> Vec<MemberRef> {
        let gkey = self.pool.intern(key);
        let shard = &self.shards[self.get_shard_index(key)];
        shard.inner.load().get(&gkey).map(|w| w.get_all()).unwrap_or_default()
    }

    /// 根据 group key 获取成员数量
    pub fn get_member_count_by_key(&self, key: &str) -> usize {
        let gkey = self.pool.intern(key);
        let shard = &self.shards[self.get_shard_index(key)];
        shard.inner.load().get(&gkey).map(|w| w.len()).unwrap_or(0)
    }
}
