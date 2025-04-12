use crate::entitys::model::FileTypeEnum;
use mongodb::bson::DateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    #[serde(rename = "_id")]
    id: String,
    pub root: bool,
    pub bucket_id: String,
    pub path_ref: String,
    pub name: String,
    pub full_path: String,
    pub file_type: FileTypeEnum,
    pub items: Vec<FileItemDto>,
    pub size: i32,
    pub create_time: DateTime
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FileItemDto{
    pub path:String,
    pub size:i32
}