use mongodb::bson::oid::ObjectId;
use mongodb::bson::DateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize,Default)]
pub struct UserRole{
    pub id: String,
    pub user_id:String,
    pub role_id:String,
    pub create_time: i64,
}

