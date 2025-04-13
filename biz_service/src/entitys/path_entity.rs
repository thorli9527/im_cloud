use mongodb::bson::DateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize,Default)]
pub struct PathInfo {
    #[serde(rename = "_id")]
    id: String,
    pub bucket_id: String,
    pub root: bool,
    pub path: String,
    pub parent_id: String,
    pub full_path: String,
    pub create_time: Option<DateTime>
}
