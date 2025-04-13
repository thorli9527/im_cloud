use mongodb::bson::oid::ObjectId;
use mongodb::bson::DateTime;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize,Default)]
pub struct UserInfo{
    #[serde(rename = "_id")]
    pub id: String,
    pub user_name:String,
    pub password:String,
    pub status:bool,
    pub is_admin:bool,
    pub create_time: Option<DateTime>,
}

