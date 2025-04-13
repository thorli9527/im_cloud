
use mongodb::bson::DateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize,Default)]
pub struct MenuInfo {
    #[serde(rename = "_id")]
    id: String,
    name:String,
    create_time: Option<DateTime>
}
