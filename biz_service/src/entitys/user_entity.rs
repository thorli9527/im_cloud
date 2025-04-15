use crate::entitys::deserialize_object_id_as_hex_string;
use crate::entitys::serialize_hex_string_as_object_id;
use mongodb::bson::DateTime;
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserInfo {
    pub id: String,
    pub user_name: String,
    pub password: String,
    pub status: bool,
    pub is_admin: bool,
    pub create_time: i64,
}
