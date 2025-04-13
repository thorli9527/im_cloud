use mongodb::bson::DateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize,Default)]
pub struct RoleMenuRel {
    #[serde(rename = "_id")]
    id: String,
    role_id:String,
    menu_id:String,
}
