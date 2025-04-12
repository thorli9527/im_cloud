use crate::errors::AppError;
use deadpool_redis::Pool;
use redis::AsyncCommands;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::any::TypeId;
#[derive(Debug, Clone)]
pub struct RedisTemplate {
   pub pool: Pool,
}

impl RedisTemplate {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    pub async fn set_key_value<T>(&self, key: impl AsRef<str>, value: T) -> Result<(), AppError>
    where
        T: Serialize + 'static,
    {
        let mut conn = self.pool.get().await?;
        if TypeId::of::<T>() == TypeId::of::<char>() {
            // SAFETY: We just checked that T is char
            let char_value = unsafe { *(std::ptr::addr_of!(value) as *const char) };
            let _: () = conn.hset(key.as_ref(), "char_value", char_value.to_string()).await?;
        } else {
            let json_value = serde_json::to_string(&value)?;
            let _: () = conn.set(key.as_ref(), json_value).await?;
        }
        Ok(())
    }
    //<'a, T>(s: &'a str)

    pub async fn get_key_value<T>(&self, key: impl AsRef<str>) -> Result<T, AppError>
    where
        T: DeserializeOwned + 'static,
    {
        let mut conn = self.pool.get().await?;
        let value: String = conn.get(key.as_ref()).await?;

        // 检查 T 是否为 char 类型
        if TypeId::of::<T>() == TypeId::of::<char>() {
            // 确保字符串长度为1
            if value.len() == 1 {
                let char_value = value.chars().next().unwrap();
                // SAFETY: 我们知道 T 是 char 类型
                let result = unsafe { std::mem::transmute_copy::<char, T>(&char_value) };
                Ok(result)
            } else {
                Err(AppError::ConversionError)
            }
        } else {
            // 对于其他类型，进行 JSON 反序列化
            let deserialized_value: T = serde_json::from_str(&value)?;
            Ok(deserialized_value)
        }
    }

    pub async fn push_to_list<T>(&self, key: impl AsRef<str>, values: Vec<T>) -> Result<(), AppError>
    where
        T: Serialize + 'static,
    {
        let mut conn = self.pool.get().await?;
        for value in values {
            let content = if let Some(char_value) = value_as_char(&value) {
                char_value.to_string()
            } else {
                serde_json::to_string(&value)?
            };
            let _: () = conn.rpush(key.as_ref(), content).await?;
        }
        Ok(())
    }

    pub async fn get_list<T>(&self, key: impl AsRef<str>) -> Result<Vec<T>, AppError>
    where
        T: DeserializeOwned + 'static,
    {
        let mut conn = self.pool.get().await?;
        let data: Vec<String> = conn.lrange(key.as_ref(), 0, -1).await?;

        // 检查 T 是否为 char 类型
        if TypeId::of::<T>() == TypeId::of::<char>() {
            let result = data
                .into_iter()
                .map(|s| s.chars().next().ok_or(AppError::ConversionError))
                .collect::<Result<Vec<char>, _>>()?;

            // SAFETY: We have already checked that T == char
            let result = unsafe {
                std::mem::transmute::<Vec<char>, Vec<T>>(result)
            };
            Ok(result)
        } else {
            // 对于其他类型，进行 JSON 反序列化
            let result = data
                .into_iter()
                .map(|s| serde_json::from_str(&s).map_err(AppError::from))
                .collect::<Result<Vec<T>, _>>()?;
            Ok(result)
        }
    }
}
fn value_as_char<T: 'static>(value: &T) -> Option<char> {
    use std::any::TypeId;
    if TypeId::of::<T>() == TypeId::of::<char>() {
        // SAFETY: We just checked that T is char
        Some(unsafe { *(value as *const T as *const char) })
    } else {
        None
    }
}