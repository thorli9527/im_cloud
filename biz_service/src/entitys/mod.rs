pub mod common_entity;
pub mod config_entity;
pub mod friend;
pub mod group_join_request;
pub mod read_index;
pub mod user_entity;

use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

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
