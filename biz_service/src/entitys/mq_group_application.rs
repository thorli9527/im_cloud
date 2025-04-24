use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub struct GroupApplication {
    pub id: String,                            // 申请记录唯一 ID（如雪花 ID 或 UUID 字符串）
    pub group_id: String,                      // 群组 ID
    pub applicant_id: String,                  // 申请人用户 ID
    pub apply_reason: Option<String>,          // 入群理由（可选）
    pub status: i8,                            // 审核状态：0 待审核 / 1 同意 / 2 拒绝
    pub reviewed_by: Option<String>,           // 审核人用户 ID（群主/管理员，可能为空）
    pub reviewed_at: i64,              // 审核时间戳（Unix 毫秒时间）
    /// 创建时间（Unix 秒时间戳）
    pub create_time: i64,
    /// 最后更新时间（Unix 秒时间戳）
    pub update_time: i64,
}
