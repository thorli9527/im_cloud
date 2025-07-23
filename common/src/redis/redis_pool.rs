// src/common/redis_pool.rs

use crate::RedisPool;
use anyhow::{Result, anyhow};
use deadpool_redis::{Config, Runtime};
use once_cell::sync::OnceCell;
use std::sync::Arc;

/// RedisPool 连接池封装，支持全局共享。
#[derive(Clone)]
pub struct RedisPoolTools {
    pub pool: Arc<RedisPool>,
}

impl RedisPoolTools {
    /// 创建 RedisPool 实例
    fn new(pool: Arc<RedisPool>) -> Self {
        Self { pool }
    }

    /// 初始化 RedisPool，全局只能初始化一次。
    ///
    /// # Panics
    /// 如果多次调用，则返回错误。
    pub fn init(redis_url: &str) -> Result<()> {
        let cfg = Config::from_url(redis_url);
        let raw_pool = cfg.create_pool(Some(Runtime::Tokio1))?;
        let instance = Self::new(Arc::new(raw_pool));
        INSTANCE.set(instance).map_err(|_| anyhow!("Redis pool already initialized"))
    }

    /// 获取全局 RedisPool 实例引用
    ///
    /// # Panics
    /// 如果未初始化，则 panic。
    pub fn get() -> &'static Arc<RedisPool> {
        &INSTANCE.get().expect("Redis pool is not initialized").pool
    }
}

// 全局 RedisPool 单例（私有）
static INSTANCE: OnceCell<RedisPoolTools> = OnceCell::new();
