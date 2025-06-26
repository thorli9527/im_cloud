use crate::entitys::agent_entity::AgentInfo;
use moka::sync::Cache;
use once_cell::sync::OnceCell;
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug)]
pub struct CacheService<T: std::clone::Clone + std::marker::Send + std::marker::Sync + 'static> {
    cache: Cache<String, T>,
}

impl<T: Clone + Send + Sync + 'static> CacheService<T> {
    /// 创建带有 TTL 和最大容量的缓存服务
    pub fn new(ttl_secs: u64, max_capacity: u64) -> Self {
        let cache = Cache::builder().time_to_live(Duration::from_secs(ttl_secs)).max_capacity(max_capacity).build();

        CacheService { cache }
    }

    /// 插入缓存项
    pub fn insert(&self, key: impl AsRef<str> + ToString, value: T) {
        self.cache.insert(key.to_string(), value);
    }

    /// 获取缓存项
    pub fn get(&self, key: impl AsRef<str> + Clone) -> Option<T> {
        let option = self.cache.get(key.as_ref());
        option
    }

    /// 删除缓存项
    pub fn remove(&self, key: &str) {
        self.cache.invalidate(key);
    }

    /// 清除所有缓存
    pub fn clear(&self) {
        self.cache.invalidate_all();
    }
}

static AGENT_CACHE_INSTANCE: OnceCell<Arc<CacheService<AgentInfo>>> = OnceCell::new();

pub fn get_agent_cache() -> Arc<CacheService<AgentInfo>> {
    AGENT_CACHE_INSTANCE.get_or_init(|| Arc::new(CacheService::new(300, 1000))).clone()
}
