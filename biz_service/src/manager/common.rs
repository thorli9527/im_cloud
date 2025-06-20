use dashmap::{DashMap, DashSet};
use fxhash::FxBuildHasher;
use serde::{Deserialize, Serialize};
// use deadpool_redis::Pool as RedisPool;
use std::str::FromStr;
use utoipa::ToSchema;

// 全局使用的ID类型，方便更改和统一类型
pub type UserId =String;
pub type GroupId = String;

// 使用 FxHash 哈希构造器的并发 Map/Set 类型别名，提升哈希性能
pub type FxDashMap<K, V> = DashMap<K, V, FxBuildHasher>;
pub type FxDashSet<T> = DashSet<T, FxBuildHasher>;

// 分片数量常量，用于将数据分片存储以减少锁竞争
pub const SHARD_COUNT: usize = 16;

// 工具函数：根据ID计算应该放入的分片索引
#[inline]
pub fn shard_index(id: u64) -> usize {
    // 使用简单取模计算分片，也可以改用其它哈希策略
    (id as usize) & (SHARD_COUNT - 1)  // 假设SHARD_COUNT是2的幂次方
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq,Default,ToSchema)]
#[repr(u8)]
#[serde(rename_all = "camelCase")]
pub enum DeviceType {
    Unknown = 0,
    #[default]
    Mobile = 1,
    Desktop = 2,
    Web = 3,
}
impl From<u8> for DeviceType {
    fn from(value: u8) -> Self {
        match value {
            1 => DeviceType::Mobile,
            2 => DeviceType::Desktop,
            3 => DeviceType::Web,
            _ => DeviceType::Unknown,
        }
    }

}

impl FromStr for DeviceType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "mobile" => Ok(DeviceType::Mobile),
            "desktop" => Ok(DeviceType::Desktop),
            "web" => Ok(DeviceType::Web),
            _ => Err(()),
        }
    }
}