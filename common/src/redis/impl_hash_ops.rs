use anyhow::Error;
use deadpool_redis::redis::cmd;

#[async_trait]
impl HashOps for HashOperations {
    /// 设置哈希字段的值（HSET）
    async fn hset<T: Serialize + Sync>(&self, key: &str, field: &str, value: &T) -> Result<()> {
        let mut conn = self.redis.pool.get().await?;
        let val = serde_json::to_string(value)?;
        let _: () = cmd("HSET").arg(key).arg(field).arg(val).query_async(&mut conn).await?;
        Ok(())
    }

    /// 获取哈希字段的值（HGET）
    async fn hget<T: DeserializeOwned + Send>(&self, key: &str, field: &str) -> Result<Option<T>> {
        let mut conn = self.redis.pool.get().await?;
        let val: Option<String> = cmd("HGET").arg(key).arg(field).query_async(&mut conn).await?;
        match val {
            Some(v) => Ok(Some(serde_json::from_str(&v)?)),
            None => Ok(None),
        }
    }

    /// 删除哈希字段（HDEL）
    async fn hdel(&self, key: &str, field: &str) -> Result<bool> {
        let mut conn = self.redis.pool.get().await?;
        let count: i64 = cmd("HDEL").arg(key).arg(field).query_async(&mut conn).await?;
        Ok(count > 0)
    }

    /// 判断哈希字段是否存在（HEXISTS）
    async fn hexists(&self, key: &str, field: &str) -> Result<bool> {
        let mut conn = self.redis.pool.get().await?;
        let exists: i64 = cmd("HEXISTS").arg(key).arg(field).query_async(&mut conn).await?;
        Ok(exists == 1)
    }

    /// 获取哈希所有字段及值（HGETALL）
    async fn hget_all<T: DeserializeOwned + Send>(&self, key: &str) -> Result<HashMap<String, T>> {
        let mut conn = self.redis.pool.get().await?;
        let raw: HashMap<String, String> = cmd("HGETALL").arg(key).query_async(&mut conn).await?;
        let parsed = raw
            .into_iter()
            .map(|(k, v)| -> Result<(String, T), Error> {
                let value = serde_json::from_str::<T>(&v)?;
                Ok((k, value))
            })
            .collect::<Result<HashMap<_, _>, _>>()?;
        Ok(parsed)
    }
    /// 获取哈希所有字段名（HKEYS）
    async fn hkeys(&self, key: &str) -> Result<Vec<String>> {
        let mut conn = self.redis.pool.get().await?;
        let keys: Vec<String> = cmd("HKEYS").arg(key).query_async(&mut conn).await?;
        Ok(keys)
    }

    /// 获取哈希所有值（HVALS）
    async fn hvals<T: DeserializeOwned + Send>(&self, key: &str) -> Result<Vec<T>> {
        let mut conn = self.redis.pool.get().await?;
        let vals: Vec<String> = cmd("HVALS").arg(key).query_async(&mut conn).await?;
        let result = vals.into_iter().map(|v| serde_json::from_str::<T>(&v)).collect::<Result<Vec<T>, _>>()?;
        Ok(result)
    }

    /// 获取哈希字段总数（HLEN）
    async fn hlen(&self, key: &str) -> Result<usize> {
        let mut conn = self.redis.pool.get().await?;
        let len: usize = cmd("HLEN").arg(key).query_async(&mut conn).await?;
        Ok(len)
    }

    /// 批量设置多个哈希字段（HMSET）
    async fn hmset<T: Serialize + Sync>(&self, key: &str, entries: &HashMap<String, T>) -> Result<()> {
        let mut conn = self.redis.pool.get().await?;

        // 将 cmd 与 arg 分离，避免临时变量生命周期问题
        let mut builder = cmd("HMSET");
        builder.arg(key);

        for (field, val) in entries {
            let val_str = serde_json::to_string(val)?;
            builder.arg(field).arg(val_str);
        }

        let _: () = builder.query_async(&mut conn).await?;
        Ok(())
    }

    /// 批量获取多个哈希字段（HMGET）
    async fn hmget<T: DeserializeOwned + Send>(&self, key: &str, fields: &[&str]) -> Result<Vec<Option<T>>> {
        let mut conn = self.redis.pool.get().await?;
        let vals: Vec<Option<String>> = cmd("HMGET").arg(key).arg(fields).query_async(&mut conn).await?;
        let result = vals
            .into_iter()
            .map(|opt| match opt {
                Some(s) => serde_json::from_str(&s).map(Some).map_err(anyhow::Error::from),
                None => Ok(None),
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(result)
    }
}
