// src/entitys/mail_entity.rs
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MailEntity {
    pub id: String,
    pub sender: String,
    pub receiver: String,
    pub subject: String,
    pub content: String,
    pub status: i32,
    pub create_time: u64,
    pub update_time: u64,
    pub send_count: i32,
}
