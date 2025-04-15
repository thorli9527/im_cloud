use mongodb::bson::DateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize,Default)]
pub struct RoleInfo {
    #[serde(rename = "_id")]
    id: String,
    name:String,
    pub create_time: i64,
}
