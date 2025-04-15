use mongodb::bson::oid::ObjectId;
use mongodb::bson::DateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize,Default)]
pub struct UserRole{
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub user_id:String,
    pub role_id:String,
    pub create_time: i64,
}

