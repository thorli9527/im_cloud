use serde::{Deserialize, Serialize};
use strum_macros::{AsRefStr, Display, EnumString};
use utoipa::ToSchema;

/// 群成员信息
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GroupMember {
    /// 成员记录唯一 ID（建议格式：group_id_user_id）
    pub id: String,
    /// 群组 ID
    pub group_id: String,
    /// 用户 ID
    pub uid: String,
    /// 成员角色（Member/Admin/Owner）
    pub role: GroupRole,
    /// 群内昵称（可选）
    pub alias: Option<String>,
    /// 是否禁言（true 禁言 / false 正常）
    pub mute: bool,
    /// 创建时间（Unix 秒时间戳）
    pub create_time: i64,
    /// 最后更新时间（Unix 秒时间戳）
    pub update_time: i64,

}

#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GroupMemberMeta {
    /// 成员记录唯一 ID（建议格式：group_id_user_id）
    pub id: String,
    /// 群组 ID
    pub group_id: String,
    /// 用户 ID
    pub uid: String,
    /// 成员角色（Member/Admin/Owner）
    pub role: GroupRole,
    /// 群内昵称（可选）
    pub alias: Option<String>,
    /// 是否禁言（true 禁言 / false 正常）
    pub mute: bool,
}

/// 群成员角色
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq, EnumString, AsRefStr, Display, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum GroupRole {
    #[default]
    Member,
    Admin,
    Owner,
}
