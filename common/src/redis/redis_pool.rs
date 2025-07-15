// src/common/redis_pool.rs
use deadpool_redis::{Config, Runtime};
use once_cell::sync::OnceCell;
use std::sync::Arc;
use anyhow::{Result, anyhow};
use deadpool_redis::redis::AsyncCommands;
use crate::RedisPool;

static REDIS_POOL: OnceCell<Arc<RedisPool>> = OnceCell::new();

/// 初始化 Redis 连接池（应在程序启动时调用一次）
pub fn init_redis_pool(redis_url: &str) -> Result<()> {
    let cfg = Config::from_url(redis_url);
    let pool = cfg.create_pool(Some(Runtime::Tokio1))?;
    REDIS_POOL.set(Arc::new(pool)).map_err(|_| anyhow!("Redis pool already initialized"))
}

/// 获取 Redis 连接池句柄（必须先调用 `init_redis_pool`）
pub fn get_redis_pool() -> Arc<RedisPool> {
    REDIS_POOL.get().expect("Redis pool is not initialized").clone()
}

/// 获取 Redis 异步连接（推荐使用此方法而非手动获取）
pub async fn get_redis_conn() -> Result<deadpool_redis::Connection> {
    let pool = get_redis_pool();
    let conn = pool.get().await?;
    Ok(conn)
}
