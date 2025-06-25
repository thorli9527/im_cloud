#[async_trait]
impl ListOps for ListOperations {
    /// 向列表右侧推入一个元素（RPUSH）
    async fn push<T: Serialize + Sync>(&self, key: &str, value: &T) -> Result<()> {
        let mut conn = self.redis.pool.get().await?;
        let val = serde_json::to_string(value)?;
        let _: () = cmd("RPUSH").arg(key).arg(val).query_async(&mut conn).await?;
        Ok(())
    }

    /// 批量向右侧推入多个元素（RPUSH）
    async fn right_push_all<T: Serialize + Sync>(&self, key: &str, values: &[T]) -> Result<()> {
        let mut conn = self.redis.pool.get().await?;
        let mut builder = cmd("RPUSH");
        builder.arg(key);
        for v in values {
            let val = serde_json::to_string(v)?;
            builder.arg(val);
        }
        let _: () = builder.query_async(&mut conn).await?;
        Ok(())
    }

    /// 向列表左侧推入一个元素（LPUSH）
    async fn left_push<T: Serialize + Sync>(&self, key: &str, value: &T) -> Result<()> {
        let mut conn = self.redis.pool.get().await?;
        let val = serde_json::to_string(value)?;
        let _: () = cmd("LPUSH").arg(key).arg(val).query_async(&mut conn).await?;
        Ok(())
    }

    /// 获取列表指定区间内的所有元素（LRANGE）
    async fn range<T: DeserializeOwned + Send>(&self, key: &str, start: isize, stop: isize) -> Result<Vec<T>> {
        let mut conn = self.redis.pool.get().await?;
        let vals: Vec<String> = cmd("LRANGE").arg(key).arg(start).arg(stop).query_async(&mut conn).await?;
        let result = vals.into_iter().map(|s| serde_json::from_str(&s)).collect::<Result<Vec<T>, _>>()?;
        Ok(result)
    }

    /// 弹出列表左侧第一个元素（LPOP）
    async fn left_pop<T: DeserializeOwned + Send>(&self, key: &str) -> Result<Option<T>> {
        let mut conn = self.redis.pool.get().await?;
        let val: Option<String> = cmd("LPOP").arg(key).query_async(&mut conn).await?;
        match val {
            Some(v) => Ok(Some(serde_json::from_str(&v)?)),
            None => Ok(None),
        }
    }

    /// 弹出列表右侧第一个元素（RPOP）
    async fn right_pop<T: DeserializeOwned + Send>(&self, key: &str) -> Result<Option<T>> {
        let mut conn = self.redis.pool.get().await?;
        let val: Option<String> = cmd("RPOP").arg(key).query_async(&mut conn).await?;
        match val {
            Some(v) => Ok(Some(serde_json::from_str(&v)?)),
            None => Ok(None),
        }
    }

    /// 获取列表长度（LLEN）
    async fn length(&self, key: &str) -> Result<usize> {
        let mut conn = self.redis.pool.get().await?;
        let len: usize = cmd("LLEN").arg(key).query_async(&mut conn).await?;
        Ok(len)
    }

    /// 删除等于指定值的元素（LREM）
    async fn remove<T: Serialize + Sync>(&self, key: &str, count: isize, value: &T) -> Result<()> {
        let mut conn = self.redis.pool.get().await?;
        let val = serde_json::to_string(value)?;
        let _: () = cmd("LREM").arg(key).arg(count).arg(val).query_async(&mut conn).await?;
        Ok(())
    }

    /// 截取列表，只保留指定区间的元素（LTRIM）
    async fn trim(&self, key: &str, start: isize, stop: isize) -> Result<()> {
        let mut conn = self.redis.pool.get().await?;
        let _: () = cmd("LTRIM").arg(key).arg(start).arg(stop).query_async(&mut conn).await?;
        Ok(())
    }
}
