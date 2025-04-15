use mongodb::bson::DateTime;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, Default,ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum MenuTypeEnum {
    #[default]
    #[serde(rename = "C")]
    FUNC,
    #[serde(rename = "F")]
    BUTTON,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MenuInfo {
    #[serde(rename = "_id")]
    pub id: String,
    pub menu_name: String,
    pub path: String,
    pub icon: String,
    pub al_icon: String,
    pub open: bool,
    pub parent_id: String,
    pub code: String,
    pub fun_type: MenuTypeEnum,
    pub new_link_flag: i32,
}
