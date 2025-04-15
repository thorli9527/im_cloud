use std::path::Path;
use mongodb::bson::DateTime;
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use strum_macros::{AsRefStr, EnumIter};
use crate::entitys::file_entity::FileTypeEnum::NORMAL;

#[derive(Debug, Clone, Serialize, Deserialize,Default)]
pub struct FileInfo {
    id: String,
    pub root: bool,
    pub bucket_id: String,
    pub path_ref: String,
    pub name: String,
    pub full_path: String,
    pub file_type: FileTypeEnum,
    pub items: Vec<FileItemDto>,
    pub size: i32,
    pub create_time: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FileItemDto{
    pub path:String,
    pub size:i32
}


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, AsRefStr, EnumIter,Default)]
#[serde(rename_all = "camelCase")]
pub enum FileTypeEnum {
    #[default]
    NORMAL,
    IMAGE,
    SVG,
    TEXT,
    DOC,
    SCRIPT,
    ZIP,
    VIDEO,
    AVI,
    MP4,
    MOV,
    MKV,
    WMV,
    FLV,
    WebM,
    RMVB,
    MPEG,
    EXCEL,
    WEBP,
    GIF,
    JPG,
    JPEG,
    PNG,
    TIFF,
    TIF,
    BMP,
}

impl FileTypeEnum {
    pub fn get_file_type(file_path: &str) -> FileTypeEnum {
        let ext = Path::new(file_path).extension().and_then(std::ffi::OsStr::to_str).unwrap_or("").to_lowercase();
        for file_type in FileTypeEnum::iter() {
            let type_name = file_type.as_ref().to_lowercase();
            if ext == type_name {
                return file_type;
            }
        }
        NORMAL
    }
}