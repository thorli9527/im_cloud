/// *
/// 群标签信息
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
#[derive(Clone, PartialEq)]
pub struct TagInfo {
    /// 标签唯一ID
    pub id: ::prost::alloc::string::String,
    /// 标签名称
    pub name: ::prost::alloc::string::String,
    /// 标签说明
    pub description: ::prost::alloc::string::String,
    /// 可选颜色代码（如 "#FF0000"）
    pub color: ::prost::alloc::string::String,
}
