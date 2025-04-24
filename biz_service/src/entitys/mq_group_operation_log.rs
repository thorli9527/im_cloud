use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub struct GroupOperationLog {
    pub id: String,                                     // 操作记录唯一 ID（可作为 MongoDB _id）
    pub group_id: String,                               // 群组 ID
    pub operator_id: String,                            // 发起操作的用户 ID
    pub operation_type: GroupOperationType,             // 操作类型（使用枚举）
    pub target_id: Option<String>,                      // 被操作用户 ID（如被踢者）
    pub extra_data: Option<HashMap<String, String>>,    // 附加参数（如禁言时长、角色等）
    /// 创建时间（Unix 秒时间戳）
    pub create_time: i64,
    /// 最后更新时间（Unix 秒时间戳）
    pub update_time: i64,
    pub sync_statue:bool                    //发送消息状态
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum GroupOperationType {
    #[default]
    Join,       // 用户加入群
    Kick,       // 将用户踢出群
    Mute,       // 禁言用户
    Unmute,     // 取消禁言
    Promote,    // 提升为管理员
    Demote,     // 撤销管理员权限
    Transfer,   // 转让群主
    Dismiss     // 群主解散群
}
