pub mod agent_entity;
pub mod client_entity;
pub mod common_entity;
pub mod config_entity;
pub mod user_entity;
pub mod group_entity;
pub mod mq_group_application;
pub mod group_member;
pub mod mq_user_action;
pub mod mq_group_operation_log;
pub mod mq_message_info;
mod read_index;

use mongodb::bson::oid::ObjectId;
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

pub fn deserialize_object_id_as_hex_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let oid = ObjectId::deserialize(deserializer)?;
    Ok(oid.to_hex())
}

// 序列化：从 String（hex） -> BSON 的 ObjectId
pub fn serialize_hex_string_as_object_id<S>(hex: &String, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let object_id = ObjectId::parse_str(hex).map_err(serde::ser::Error::custom)?;
    object_id.serialize(serializer)
}
