use mongodb::bson::DateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize,Default)]
pub struct RoleInfo {
    id: String,
    name:String,
    pub create_time: i64,
}
