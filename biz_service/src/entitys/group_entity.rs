use crate::protocol::common::JoinPermission;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub struct GroupEntity {
    pub id: String,                           // 群组唯一 ID（可为 UUID 或雪花 ID）
    pub name: String,                         // 群名称（展示给用户）
    pub avatar: Option<String>,               // 群组头像 URL
    pub description: Option<String>,          // 群组简介
    pub notice: Option<String>,         // 群组公告
    pub owner_id: String,                     // 群创拥有用户 ID
    pub group_type: i32,                      // 群类型：0 普通群 / 1 超级群 / 2 系统群
    pub join_permission: JoinPermission,      // 加入权限：0 所有人可加入 /
    pub allow_search: bool,                   // 是否允许通过搜索找到
    pub enable: bool,                           // 是否启用
    pub create_time: i64,                     // 创建时间（Unix 秒时间戳）
    pub update_time: i64,                     // 最后更新时间（Unix 秒时间戳）
}