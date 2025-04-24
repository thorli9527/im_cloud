use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub struct GroupMember {
    pub id: String,                            // 成员记录唯一 ID（建议格式：group_id_user_id）
    pub group_id: String,                      // 群组 ID
    pub user_id: String,                       // 用户 ID
    pub role: GroupRole,                       // 成员角色：枚举类型，更安全可读
    pub alias: Option<String>,                 // 群内昵称（用户可设置）
    pub mute_until: i64,               // 禁言截止时间（Unix 毫秒时间戳）
    /// 创建时间（Unix 秒时间戳）
    pub create_time: i64,
    /// 最后更新时间（Unix 秒时间戳）
    pub update_time: i64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, PartialEq, Eq,Default)]
#[serde(rename_all = "lowercase")]
pub enum GroupRole {
    #[default]
    Member,

    Admin,

    Owner,
}