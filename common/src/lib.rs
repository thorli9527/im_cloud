extern crate core;

pub mod config;
pub mod errors;
pub mod models;
pub mod redis;
pub mod repository;
pub mod util;

pub use repository::*;
use serde::{Deserialize, Serialize};
pub type UserId = String;
pub type GroupId = String;
pub type MessageId = u64;
pub type RedisPool = deadpool_redis::Pool;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClientTokenDto {
    pub uid: String,
    pub device_type: u8,
}
