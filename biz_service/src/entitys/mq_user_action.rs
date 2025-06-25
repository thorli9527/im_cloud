use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;
#[derive(Debug, Clone,  Serialize, Deserialize, ToSchema, Default)]
pub struct UserActionLog {
    pub id: String,  
    pub agent_id:String,// 商户id
    pub uid: String,                        // 被操作用户
    pub action: UserActionType,              // 动作类型（使用枚举）
    pub reason: Option<String>,              // 操作原因（如违规内容）
    pub remark: Option<String>,              // 备注说明（如后台记录、系统生成信息）
    pub operator_id: String,            // 操作者（系统 or 管理员）
    pub expired_at: Option<i64>,   // 动作的过期时间（禁言/封号）
    pub extra_data: Option<HashMap<String, String>>,        // 扩展字段（如 IP、设备、上下文信息）
    pub sync_statue:bool,                   //发送消息状态
    /// 创建时间（Unix 秒时间戳）
    pub create_time: i64,
    /// 最后更新时间（Unix 秒时间戳）
    pub update_time: i64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum UserActionType {
    #[default]
    Knockout,    // 强制下线
    Mute,       // 禁言
    Unmute,     // 解禁
    Ban,        // 封号
    Unban,      // 解封
    Login,      // 登录
    Logout,     // 登出
    Warn,       // 警告
    Block,      // 拉黑
    Unblock,    // 取消拉黑
    Report,     // 举报
}