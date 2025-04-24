use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// 国家信息结构体，用于存储标准的国家相关基础数据
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub struct Country {
    /// 主键 ID
    pub id: i32,

    /// 国家英文名称，如 "China"
    pub name_en: String,

    /// 国家本地语言名称，如 "中国"，可选
    pub name_local: Option<String>,

    /// 国家 ISO 3166-1 alpha-2 两位代码，如 "CN"
    pub iso_code: String,

    /// 国家 ISO 3166-1 alpha-3 三位代码，如 "CHN"
    pub iso_code3: Option<String>,

    /// 国家电话区号，如 "+86"
    pub phone_code: Option<String>,

    /// 货币代码，如 "CNY"
    pub currency_code: Option<String>,

    /// 国旗图标 URL，可选
    pub flag_url: Option<String>,

    /// 是否启用该国家，适合做国家列表筛选
    pub is_active: bool,
    /// 创建时间（Unix 秒时间戳）
    pub create_time: i64,
    /// 最后更新时间（Unix 秒时间戳）
    pub update_time: i64,
}
