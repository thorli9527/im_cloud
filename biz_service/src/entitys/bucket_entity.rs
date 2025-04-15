use mongodb::bson::DateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize,Default)]
pub struct BucketInfo {
    pub id: String,
    pub name: String,
    pub path:String,
    pub quota: i32,
    pub current_quota: i32,
    pub pub_read: bool,
    pub pub_write: bool,
    pub create_time: i64,
}