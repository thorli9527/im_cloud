#![allow(unused_variables)]
#![allow(dead_code)]

use hex::encode;
use std::hash::{Hash, Hasher};
use std::time::{SystemTime, UNIX_EPOCH};

use md5::{Digest, Md5};
use rand::distr::Alphanumeric;
use serde::{Deserialize, Serialize};
use twox_hash::XxHash64;
use uuid::Uuid;
use rand::Rng;
pub fn copy_to<A, B>(a: &A, b: &B) -> B
where
    A: Serialize + for<'de> Deserialize<'de>,
    B: Serialize + for<'de> Deserialize<'de>,
{
    serde_json::from_value(serde_json::to_value(a).unwrap()).unwrap()
}

pub fn build_uuid() -> String {
    let uuid = Uuid::new_v4().simple();
    format!("{}", uuid)
}
pub fn build_snow_id() -> u64 {
    let mut generator = SafeSnowflake::new(1, 1);
    return generator.generate();
}
// 计算字符串的哈希值并返回在指定范围内的索引
pub fn hash_index(key: &str, total: i32) -> i32 {
    let mut hasher1 = XxHash64::with_seed(0);
    key.hash(&mut hasher1);
    hasher1.finish() as i32 % total
}
pub fn build_md5(content: &str) -> String {
    let mut hasher = Md5::new();
    hasher.update(content);
    let result = hasher.finalize();
    let hex_string = encode(result);
    hex_string
}
pub fn build_md5_with_key(content: &str, key: &str) -> String {
    let mut hasher = Md5::new();
    hasher.update(format!("{},{}", content, key));
    let result = hasher.finalize();
    let hex_string = encode(result);
    hex_string
}


pub fn build_uid() -> String {
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::rng();

    (0..16)
        .map(|_| {
            let i = rng.random_range(0..CHARSET.len());
            CHARSET[i] as char
        })
        .collect()
}

pub struct SafeSnowflake {
    node_id: u64,
    worker_id: u64,
    sequence: u64,
    last_timestamp: u64,
}

impl SafeSnowflake {
    pub fn new(node_id: u64, worker_id: u64) -> Self {
        Self {
            node_id: node_id & 0x1F,     // 5 bits
            worker_id: worker_id & 0x1F, // 5 bits
            sequence: 0,
            last_timestamp: 0,
        }
    }

    fn current_timestamp() -> u64 {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        now.as_millis() as u64 // 毫秒时间戳
    }

    pub fn generate(&mut self) -> u64 {
        let mut timestamp = Self::current_timestamp();

        if timestamp == self.last_timestamp {
            self.sequence = (self.sequence + 1) & 0b11; // 2 bits
            if self.sequence == 0 {
                // 同一毫秒内超出最大序列，等下一毫秒
                while timestamp <= self.last_timestamp {
                    timestamp = Self::current_timestamp();
                }
            }
        } else {
            self.sequence = 0;
        }

        self.last_timestamp = timestamp;

        // 拼接为 53 位以内的 ID:
        // 41 bits timestamp | 5 bits node_id | 5 bits worker_id | 2 bits sequence
        ((timestamp & 0x1FFFFFFFFFF) << 12) // 41 bits
            | ((self.node_id & 0x1F) << 7)  // 5 bits
            | ((self.worker_id & 0x1F) << 2) // 5 bits
            | (self.sequence & 0x03) // 2 bits
    }
}


