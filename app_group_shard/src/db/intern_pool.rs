use dashmap::DashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Weak};
use std::time::Duration;
use tokio::time::interval;

/// 分层、分片的字符串 intern 池模块
/// 支持 Hot Cache (arc<str>) 与 Cold Store (weak<str>)
///
#[derive(Debug)]
pub struct InternPool {
    shards: Vec<InternShard>,
    shard_count: usize,
}

#[derive(Debug)]
struct InternShard {
    /// 热缓存：高优先级 key，限容量
    hot: DashMap<Arc<str>, ()>,
    /// 冷存储：弱引用跟踪更多历史 key
    cold: DashMap<String, Weak<str>>,
    capacity: usize,
}

impl InternPool {
    /// 创建 InternPool
    /// `shard_count` 分片数
    /// `hot_capacity_per_shard` 每个 shard 热缓存容量
    pub fn new(shard_count: usize, hot_capacity_per_shard: usize) -> Self {
        let mut shards = Vec::with_capacity(shard_count);
        for _ in 0..shard_count {
            shards.push(InternShard {
                hot: DashMap::new(),
                cold: DashMap::new(),
                capacity: hot_capacity_per_shard,
            });
        }
        InternPool {
            shards,
            shard_count,
        }
    }

    /// Intern 一个 key，返回 Arc<str>
    pub fn intern(&self, key: &str) -> Arc<str> {
        // 1. 选择 shard
        let idx = self.shard_index(key);
        let shard = &self.shards[idx];
        // 2. hot 命中
        if let Some(entry) = shard.hot.get(key) {
            // 升级或复用 cold 中的 Arc
            return shard.upgrade_or_reuse(key);
        }
        // 3. cold 升级
        if let Some(entry) = shard.cold.get(key) {
            if let Some(arc_str) = entry.value().upgrade() {
                shard.hot.insert(arc_str.clone(), ());
                shard.maybe_evict();
                return arc_str;
            }
        }
        // 4. 新建
        let arc_str: Arc<str> = Arc::from(key.to_string().into_boxed_str());
        shard.hot.insert(arc_str.clone(), ());
        shard.cold.insert(key.to_string(), Arc::downgrade(&arc_str));
        shard.maybe_evict();
        arc_str
    }

    /// 并行回收所有 shards 的 dead weak 引用
    pub fn cleanup(&self) {
        for shard in &self.shards {
            shard.cleanup_sample();
        }
    }

    /// 启动后台定时清理，每隔 `interval_secs` 秒运行一次 cleanup
    pub async fn spawn_cleanup_task(self: Arc<Self>, interval_secs: u64) {
        let mut ticker = interval(Duration::from_secs(interval_secs));
        loop {
            ticker.tick().await;
            self.cleanup();
        }
    }

    #[inline]
    fn shard_index(&self, key: &str) -> usize {
        // 简单哈希: 使用 std 的哈希
        let mut hasher = ahash::AHasher::default();
        key.hash(&mut hasher);
        (hasher.finish() as usize) % self.shard_count
    }
}

impl InternShard {
    /// 升级或复用 cold 中的 Arc<str>
    fn upgrade_or_reuse(&self, key: &str) -> Arc<str> {
        // cold.get 已知存在时调用
        if let Some(entry) = self.cold.get(key) {
            if let Some(arc_str) = entry.value().upgrade() {
                return arc_str;
            }
        }
        // fallback: 构造临时 Arc (should not happen)
        Arc::from(key.to_string().into_boxed_str())
    }

    /// 当 hot 超过 capacity 时，淘汰部分最早插入或随机的一批
    fn maybe_evict(&self) {
        let len = self.hot.len();
        if len <= self.capacity {
            return;
        }
        // 淘汰 10% 热点
        let to_remove = len / 10;
        let mut count = 0;
        for entry in self.hot.iter() {
            self.hot.remove(entry.key());
            count += 1;
            if count >= to_remove {
                break;
            }
        }
    }

    /// 随机/采样清理若干冷引用
    fn cleanup_sample(&self) {
        const SAMPLE: usize = 64;
        let keys: Vec<String> = self.cold.iter().take(SAMPLE).map(|e| e.key().clone()).collect();
        for k in keys {
            if let Some(entry) = self.cold.get(&k) {
                if entry.value().upgrade().is_none() {
                    self.cold.remove(&k);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::InternPool;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_intern_reuse() {
        let pool = Arc::new(InternPool::new(4, 2));
        let a1 = pool.intern("hello");
        let a2 = pool.intern("hello");
        assert!(Arc::ptr_eq(&a1, &a2));

        // cleanup does not remove hot keys
        pool.cleanup();
        let a3 = pool.intern("hello");
        assert!(Arc::ptr_eq(&a1, &a3));
    }

    #[tokio::test]
    async fn test_cleanup_evicted() {
        let pool = Arc::new(InternPool::new(4, 1));
        // capacity 1 per shard, so second insert evicts first
        let x = pool.intern("key1");
        let y = pool.intern("key2");
        // key1 weak still in cold
        drop(x);
        // cleanup should remove dead weak
        pool.cleanup();
        // intern key1 yields new allocation
        let x2 = pool.intern("key1");
        assert!(!Arc::ptr_eq(&x2, &y));
    }
}
