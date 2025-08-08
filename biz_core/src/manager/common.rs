// 全局使用的ID类型，方便更改和统一类型

pub type GroupId = String;

// 使用 FxHash 哈希构造器的并发 Map/Set 类型别名，提升哈希性能

// 分片数量常量，用于将数据分片存储以减少锁竞争
pub const SHARD_COUNT: usize = 16;

// 工具函数：根据ID计算应该放入的分片索引
#[inline]
pub fn shard_index(id: u64) -> usize {
    // 使用简单取模计算分片，也可以改用其它哈希策略
    (id as usize) & (SHARD_COUNT - 1) // 假设SHARD_COUNT是2的幂次方
}
