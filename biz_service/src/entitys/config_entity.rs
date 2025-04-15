
use mongodb::bson::DateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize,Default)]
pub struct ConfigInfo {
    #[serde(rename = "_id")]
    id: String,
    name:String,
    config_type:ConfigTypeEnum,
    pub create_time: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize,Default)]
pub enum ConfigTypeEnum{
    #[default]
    BlockSize
}