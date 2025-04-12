use crate::entitys::model::FileTypeEnum::NORMAL;
use serde::{Deserialize, Serialize};
use std::path::Path;
use strum::IntoEnumIterator;
use strum_macros::{AsRefStr, EnumIter};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, AsRefStr, EnumIter)]
#[serde(rename_all = "camelCase")]
pub enum FileTypeEnum {
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
