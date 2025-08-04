use crate::db::member::member_list_wrapper::MemberListWrapper;
use crate::error::member_list_error::MemberListError;
use arc_swap::ArcSwap;
use biz_service::protocol::common::GroupRoleType;
use biz_service::protocol::rpc::arb_models::MemberRef;
use rand::{rng, Rng};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use twox_hash::XxHash64;

#[derive(Debug)]
struct Shard {
    inner: ArcSwap<HashMap<String, Arc<MemberListWrapper>>>, // lock-free COW map per shard
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
    /// legacy 兼容字段：每 group 期望的 shard 用途（如果不再手动驱动可以删掉）
    pub per_group_shard: usize,
}

impl HashShardMap {
    pub fn new(shard_count: usize, per_group_shard: usize) -> Self {
        let shards = (0..shard_count).map(|_| Shard::default()).collect();
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

    /// 获取或创建对应 key 的 MemberListWrapper（无锁，用 CAS 循环插入）
    fn get_or_create_wrapper(&self, key: &str) -> Arc<MemberListWrapper> {
        let shard = &self.shards[self.get_shard_index(key)];
        loop {
            let current = shard.inner.load_full(); // Arc<HashMap<...>>
            if let Some(existing) = current.get(key) {
                return existing.clone();
            }
            // 插入新 wrapper（CAS copy-on-write）
            let mut new_map = (*current).clone();
            let wrapper = Arc::new(MemberListWrapper::new_simple());
            new_map.insert(key.to_string(), wrapper.clone());
            let new_arc = Arc::new(new_map);
            let prev = shard.inner.compare_and_swap(&current, new_arc);
            if Arc::ptr_eq(&prev, &current) {
                return wrapper;
            }
            // 否则重试
        }
    }
    /// 通用重试工具函数，支持指数退避（Exponential Backoff）与随机抖动（Jitter）。
    /// 用于临时性错误（如 Redis 写冲突、乐观锁冲突等）场景的自动重试。
    ///
    /// # 参数
    /// - `op`: 待执行的闭包操作，返回 `Result<T, MemberListError>`
    ///         若返回 `Ok(T)` 表示成功，立即返回结果；
    ///         若返回 `Err(MemberListError::Retry)` 则触发重试机制；
    ///         若返回其他错误类型则立即返回该错误。
    ///
    /// # 返回值
    /// - 成功：返回操作闭包的 `Ok(T)`
    /// - 重试失败：达到最大重试次数后仍失败，则返回 `Err(MemberListError::Retry)`
    /// - 非重试错误：直接返回 `op()` 返回的其他 `Err(e)`
    ///
    /// # 行为说明
    /// - 初始等待时间为 `INITIAL_BACKOFF_MS` 毫秒（100ms）
    /// - 每次失败后，等待时间指数增长（乘以2），最大不超过 `MAX_BACKOFF_MS`（1000ms）
    /// - 在每次等待中加入随机抖动（jitter）以避免雪崩式重试
    fn retry_op<T>(&self, mut op: impl FnMut() -> Result<T, MemberListError>) -> Result<T, MemberListError> {
        const MAX_RETRIES: usize = 5; // 最大重试次数
        const INITIAL_BACKOFF_MS: u64 = 100; // 初始退避时间：100ms
        const MAX_BACKOFF_MS: u64 = 1000; // 最大退避上限：1秒

        let mut backoff = INITIAL_BACKOFF_MS;

        for attempt in 0..MAX_RETRIES {
            match op() {
                Ok(res) => return Ok(res), // 成功，直接返回结果
                Err(MemberListError::Retry) => {
                    if attempt == MAX_RETRIES - 1 {
                        break; // 最后一次失败后不再继续
                    }
                    self.sleep_with_jitter(backoff); // 加入退避等待
                    backoff = (backoff * 2).min(MAX_BACKOFF_MS); // 指数增长并限制最大值
                }
                Err(e) => return Err(e), // 非重试错误，直接返回
            }
        }
        Err(MemberListError::Retry) // 达到最大重试次数仍失败，返回最终错误
    }

    /// 在基础退避时间上加入随机抖动，避免集群中多个节点同步重试导致雪崩效应。
    ///
    /// # 参数
    /// - `base_ms`: 当前退避的基础毫秒数（如100、200等）
    ///
    /// # 行为
    /// - 在 `base_ms` 基础上，额外增加 `[0, base_ms)` 范围的随机毫秒数
    /// - 最终实际等待时间为 `base_ms + jitter`
    /// - 使用阻塞式 `thread::sleep`，若在异步环境中应改为 `tokio::time::sleep`
    fn sleep_with_jitter(&self, base_ms: u64) {
        let mut rng = rng(); // 使用全局 rng() 工具函数（可能是 thread_local 封装）
        let jitter = rng.gen_range(0..base_ms); // 随机抖动：0 ~ base_ms - 1
        let dur = Duration::from_millis(base_ms + jitter);
        thread::sleep(dur); // 阻塞等待
    }

    pub fn insert(&self, key: String, member: MemberRef) -> Result<(), MemberListError> {
        if key.is_empty() {
            return Ok(());
        }
        let wrapper = self.get_or_create_wrapper(&key);
        self.retry_op(|| wrapper.add(member.clone()))
    }

    pub fn insert_many(&self, key: &str, members: Vec<MemberRef>) -> Result<(), MemberListError> {
        if key.is_empty() {
            return Ok(());
        }
        let wrapper = self.get_or_create_wrapper(key);
        self.retry_op(|| wrapper.add_many(members.clone()))
    }

    pub fn get_page(&self, key: &str, page: usize, page_size: usize) -> Option<Vec<MemberRef>> {
        let shard = &self.shards[self.get_shard_index(key)];
        shard.inner.load().get(key).map(|entry| entry.get_page(page, page_size))
    }

    pub fn get_online_ids(&self, key: &str) -> Vec<String> {
        let shard = &self.shards[self.get_shard_index(key)];
        shard.inner.load().get(key).map(|entry| entry.get_online_ids()).unwrap_or_default()
    }

    pub fn get_online_page(&self, key: &str, page: usize, page_size: usize) -> Vec<String> {
        let shard = &self.shards[self.get_shard_index(key)];
        shard.inner.load().get(key).map(|entry| entry.get_online_page(page, page_size)).unwrap_or_default()
    }

    pub fn get_online_count(&self, key: &str) -> usize {
        let shard = &self.shards[self.get_shard_index(key)];
        shard.inner.load().get(key).map(|entry| entry.get_online_count()).unwrap_or(0)
    }

    pub fn remove(&self, key: &str, user_id: &str) -> Result<bool, MemberListError> {
        let shard = &self.shards[self.get_shard_index(key)];
        if let Some(wrapper) = shard.inner.load().get(key) {
            // 使用 retry helper 让逻辑一致
            return self.retry_op(|| wrapper.remove(user_id));
        }
        Ok(false)
    }

    pub fn set_online(&self, key: &str, user_id: &str, online: bool) -> Result<(), MemberListError> {
        let shard = &self.shards[self.get_shard_index(key)];
        if let Some(entry) = shard.inner.load().get(key) {
            return self.retry_op(|| entry.set_online(user_id, online));
        }
        Ok(())
    }

    pub fn change_role(&self, key: &str, user_id: &str, role: GroupRoleType) -> Result<(), MemberListError> {
        let shard = &self.shards[self.get_shard_index(key)];
        if let Some(entry) = shard.inner.load().get(key) {
            return self.retry_op(|| entry.change_role(user_id, role));
        }
        Ok(())
    }

    /// 所有 group key（可能很多，慎用）
    pub fn all_keys(&self) -> Vec<String> {
        self.shards.iter().flat_map(|shard| shard.inner.load().keys().cloned().collect::<Vec<_>>()).collect()
    }

    pub fn get_member_by_key(&self, key: &str) -> Vec<MemberRef> {
        let shard = &self.shards[self.get_shard_index(key)];
        shard.inner.load().get(key).map(|entry| entry.get_all()).unwrap_or_default()
    }

    pub fn get_member_count_by_key(&self, key: &str) -> usize {
        let shard = &self.shards[self.get_shard_index(key)];
        shard.inner.load().get(key).map(|entry| entry.len()).unwrap_or(0)
    }

    /// 清掉某个 key 的 wrapper（整个 group 清除）
    pub fn clear(&self, key: &str) {
        let shard = &self.shards[self.get_shard_index(key)];
        loop {
            let current = shard.inner.load_full();
            if !current.contains_key(key) {
                return;
            }
            let mut new_map = (*current).clone();
            new_map.remove(key);
            let new_arc = Arc::new(new_map);
            let prev = shard.inner.compare_and_swap(&current, new_arc);
            if Arc::ptr_eq(&prev, &current) {
                return;
            }
        }
    }
}
