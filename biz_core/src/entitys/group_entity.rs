/// *
/// 群组基本信息（用于展示和配置）
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
#[derive(Clone, PartialEq, Debug)]
pub struct GroupEntity {
    /// 群组唯一ID（由系统生成）
    pub id: ::prost::alloc::string::String,
    /// 群组名称（用户可见）
    pub name: ::prost::alloc::string::String,
    /// 群头像URL
    pub avatar: ::prost::alloc::string::String,
    /// 群简介（支持富文本）
    pub description: ::prost::alloc::string::String,
    /// 群公告（群成员可见）
    pub notice: ::prost::alloc::string::String,
    /// 加群权限控制
    pub join_permission: i32,
    /// 群标签（英文逗号分隔）
    pub owner_id: ::prost::alloc::string::String,
    /// 群组类型
    pub group_type: i32,
    /// 是否允许通过搜索找到
    pub allow_search: bool,
    /// 是否启用
    pub enable: bool,
    /// 创建时间
    pub create_time: u64,
    /// 更新时间
    pub update_time: u64,
}
