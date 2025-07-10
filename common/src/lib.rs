pub mod config;
pub mod db;
pub mod errors;
pub mod redis;
pub mod repository;
pub mod util;

pub use repository::*;
use serde::{Deserialize, Serialize};
pub type UserId = String;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClientTokenDto {
    pub agent_id: String,
    pub uid: String,
    pub device_type: u8,
}
