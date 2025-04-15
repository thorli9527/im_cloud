use mongodb::bson::DateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RoleMenuRel {
    pub id: String,
    pub role_id: String,
    pub menu_id: String,
}
