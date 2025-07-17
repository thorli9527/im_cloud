use crate::redis::redis_pool::RedisPoolTools;
use crate::RedisPool;
use anyhow::Result;
use async_trait::async_trait;
use deadpool_redis::redis;
use deadpool_redis::redis::Pipeline;
use once_cell::sync::OnceCell;
use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// RedisTemplate 是 Redis 操作的统一入口点，封装了连接池并提供 Value/List/Hash 操作接口
#[derive(Clone, Debug)]
pub struct RedisTemplate {
    pub pool: Arc<RedisPool>,
}

impl RedisTemplate {
    fn new() -> Self {
        let pool =  RedisPoolTools::get().clone();
        RedisTemplate { pool }
    }

    pub fn init() {
        let instance = Self::new();
        INSTANCE.set(Arc::new(instance)).expect("AgentService already initialized");
    }

    pub fn get() -> Arc<Self> {
        INSTANCE.get().expect("GroupManager not initialized").clone()
    }
    /// 获取 Value 类型操作接口实现
    pub fn ops_for_value(self: &Arc<Self>) -> impl ValueOps {
        ValueOperations { redis: self.clone() }
    }

    /// 获取 List 类型操作接口实现
    pub fn ops_for_list(self: &Arc<Self>) -> impl ListOps {
        ListOperations { redis: self.clone() }
    }

    /// 获取 Hash 类型操作接口实现
    pub fn ops_for_hash(self: &Arc<Self>) -> impl HashOps {
        HashOperations { redis: self.clone() }
    }

    /// 执行一组命令作为 pipeline（非事务）
    /// 执行一组命令作为 pipeline（非事务）
    pub async fn execute_pipeline<F>(&self, pipe_fn: F) -> Result<()>
    where
        F: FnOnce(&mut Pipeline) -> &mut Pipeline + Send,
    {
        let mut conn = self.pool.get().await?;
        let mut pipe = redis::pipe();
        pipe_fn(&mut pipe);
        let _: () = pipe.query_async(&mut conn).await?;
        Ok(())
    }

    /// 执行一组命令作为事务（MULTI/EXEC）
    pub async fn execute_transaction<F>(&self, _keys: &[&str], txn_fn: F) -> Result<()>
    where
        F: FnOnce(&mut Pipeline) -> &mut Pipeline + Send,
    {
        let mut conn = self.pool.get().await?;
        let mut pipe = redis::pipe();
        pipe.atomic();
        txn_fn(&mut pipe);
        let _: () = pipe.query_async(&mut conn).await?;
        Ok(())
    }
}

/// Value 类型 Redis 操作接口定义
#[async_trait]
pub trait ValueOps {
    async fn set<T: Serialize + Sync>(&self, key: &str, value: &T, expire: Option<u64>) -> Result<()>;
    async fn get<T: DeserializeOwned + Send>(&self, key: &str) -> Result<Option<T>>;
    async fn delete(&self, key: &str) -> Result<bool>;
    async fn has_key(&self, key: &str) -> Result<bool>;
    async fn increment(&self, key: &str, delta: i64) -> Result<i64>;
    async fn decrement(&self, key: &str, delta: i64) -> Result<i64>;
    async fn expire(&self, key: &str, seconds: u64) -> Result<bool>;
    async fn ttl(&self, key: &str) -> Result<i64>;
}

/// List 类型 Redis 操作接口定义
#[async_trait]
pub trait ListOps {
    async fn push<T: Serialize + Sync>(&self, key: &str, value: &T) -> Result<()>;
    async fn right_push_all<T: Serialize + Sync>(&self, key: &str, values: &[T]) -> Result<()>;
    async fn left_push<T: Serialize + Sync>(&self, key: &str, value: &T) -> Result<()>;
    async fn range<T: DeserializeOwned + Send>(&self, key: &str, start: isize, stop: isize) -> Result<Vec<T>>;
    async fn left_pop<T: DeserializeOwned + Send>(&self, key: &str) -> Result<Option<T>>;
    async fn right_pop<T: DeserializeOwned + Send>(&self, key: &str) -> Result<Option<T>>;
    async fn length(&self, key: &str) -> Result<usize>;
    async fn remove<T: Serialize + Sync>(&self, key: &str, count: isize, value: &T) -> Result<()>;
    async fn trim(&self, key: &str, start: isize, stop: isize) -> Result<()>;
}

/// Hash 类型 Redis 操作接口定义
#[async_trait]
pub trait HashOps {
    async fn hset<T: Serialize + Sync>(&self, key: &str, field: &str, value: &T) -> Result<()>;
    async fn hget<T: DeserializeOwned + Send>(&self, key: &str, field: &str) -> Result<Option<T>>;
    async fn hdel(&self, key: &str, field: &str) -> Result<bool>;
    async fn hexists(&self, key: &str, field: &str) -> Result<bool>;
    async fn hget_all<T: DeserializeOwned + Send>(&self, key: &str) -> Result<HashMap<String, T>>;
    async fn hkeys(&self, key: &str) -> Result<Vec<String>>;
    async fn hvals<T: DeserializeOwned + Send>(&self, key: &str) -> Result<Vec<T>>;
    async fn hlen(&self, key: &str) -> Result<usize>;
    async fn hmset<T: Serialize + Sync>(&self, key: &str, entries: &HashMap<String, T>) -> Result<()>;
    async fn hmget<T: DeserializeOwned + Send>(&self, key: &str, fields: &[&str]) -> Result<Vec<Option<T>>>;
}

/// Value 类型接口实现结构体
pub struct ValueOperations {
    pub redis: Arc<RedisTemplate>,
}

/// List 类型接口实现结构体
pub struct ListOperations {
    pub redis: Arc<RedisTemplate>,
}

/// Hash 类型接口实现结构体
pub struct HashOperations {
    pub redis: Arc<RedisTemplate>,
}

// ✅ 接口实现定义，建议拆分为独立文件维护
include!("impl_value_ops.rs");
include!("impl_list_ops.rs");
include!("impl_hash_ops.rs");

// 单例静态变量
static INSTANCE: OnceCell<Arc<RedisTemplate>> = OnceCell::new();
