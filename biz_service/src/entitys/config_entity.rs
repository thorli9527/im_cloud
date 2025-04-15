
use mongodb::bson::DateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize,Default)]
pub struct ConfigInfo {
    pub id: String,
    pub name:String,
    pub config_type:ConfigTypeEnum,
    pub create_time: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize,Default)]
pub enum ConfigTypeEnum{
    #[default]
    BlockSize
}