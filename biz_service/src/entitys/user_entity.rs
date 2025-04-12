use mongodb::bson::oid::ObjectId;
use mongodb::bson::DateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize,Default)]
pub struct UserInfo{
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub user_name:Option<String>,
    pub password:String,
    pub status:Option<bool>,
    pub is_admin:Option<bool>,
    pub create_time: Option<DateTime>
}

