#[async_trait]
impl ValueOps for ValueOperations {
    /// 设置键值，支持可选过期时间（单位：秒）
    async fn set<T: Serialize + Sync>(&self, key: &str, value: &T, expire: Option<u64>) -> Result<()> {
        let mut conn = self.redis.pool.get().await?;
        let data = serde_json::to_string(value)?;
        match expire {
            Some(ttl) => {
                let _: () = cmd("SETEX").arg(key).arg(ttl).arg(data).query_async(&mut conn).await?;
            }
            None => {
                let _: () = cmd("SET").arg(key).arg(data).query_async(&mut conn).await?;
            }
        }
        Ok(())
    }

    /// 获取指定 key 的值，并反序列化为目标类型
    async fn get<T: DeserializeOwned + Send>(&self, key: &str) -> Result<Option<T>> {
        let mut conn = self.redis.pool.get().await?;
        let val: Option<String> = cmd("GET").arg(key).query_async(&mut conn).await?;
        match val {
            Some(v) => Ok(Some(serde_json::from_str(&v)?)),
            None => Ok(None),
        }
    }

    /// 删除指定 key，返回是否删除成功
    async fn delete(&self, key: &str) -> Result<bool> {
        let mut conn = self.redis.pool.get().await?;
        let deleted: i64 = cmd("DEL").arg(key).query_async(&mut conn).await?;
        Ok(deleted > 0)
    }

    /// 判断指定 key 是否存在
    async fn has_key(&self, key: &str) -> Result<bool> {
        let mut conn = self.redis.pool.get().await?;
        let exists: i64 = cmd("EXISTS").arg(key).query_async(&mut conn).await?;
        Ok(exists > 0)
    }

    /// 对 key 对应的整数值自增（默认步长 1）
    async fn increment(&self, key: &str, delta: i64) -> Result<i64> {
        let mut conn = self.redis.pool.get().await?;
        let result: i64 = cmd("INCRBY").arg(key).arg(delta).query_async(&mut conn).await?;
        Ok(result)
    }

    /// 对 key 对应的整数值自减（默认步长 1）
    async fn decrement(&self, key: &str, delta: i64) -> Result<i64> {
        let mut conn = self.redis.pool.get().await?;
        let result: i64 = cmd("DECRBY").arg(key).arg(delta).query_async(&mut conn).await?;
        Ok(result)
    }

    /// 设置指定 key 的过期时间（单位：秒）
    async fn expire(&self, key: &str, seconds: u64) -> Result<bool> {
        let mut conn = self.redis.pool.get().await?;
        let result: i64 = cmd("EXPIRE").arg(key).arg(seconds).query_async(&mut conn).await?;
        Ok(result == 1)
    }

    /// 获取指定 key 剩余存活时间（单位：秒；-1 表示永久）
    async fn ttl(&self, key: &str) -> Result<i64> {
        let mut conn = self.redis.pool.get().await?;
        let ttl: i64 = cmd("TTL").arg(key).query_async(&mut conn).await?;
        Ok(ttl)
    }
}
