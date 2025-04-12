use mongodb::bson::DateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketInfo {
    #[serde(rename = "_id")]
    id: String,
    pub name: String,
    pub quota: i32,
    pub current_quota: i32,
    pub pub_read: bool,
    pub pub_write: bool,
    pub create_time: DateTime
}