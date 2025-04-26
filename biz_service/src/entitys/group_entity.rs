use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub struct GroupInfo {
    pub id: String,                         // 群组唯一 ID（可为 UUID 或雪花 ID）
    pub group_id:String,                    //三方群id
    pub agent_id:String,                    // 商户id
    pub name: String,                       // 群名称（展示给用户）
    pub icon_url: Option<String>,           // 群头像链接（可选）
    pub notice: Option<String>,             // 群公告内容（可选）
    pub creator_id: String,                    // 群创建者用户 ID
    pub group_type: i32,                     // 群类型：0 普通群 / 1 超级群 / 2 系统群
    pub max_members: i32,                   // 最大成员限制（默认如 500、2000 等）
    pub status: i8,                         // 群状态：1 正常 / 0 已解散（逻辑删除）
    /// 创建时间（Unix 秒时间戳）
    pub create_time: i64,
    /// 最后更新时间（Unix 秒时间戳）
    pub update_time: i64,
}




