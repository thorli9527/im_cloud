use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub struct UserFriend {
    pub id: String,                             // 用户 ID
    pub agent_id: String,                       // 代理 ID
    pub uid: String,                            // 用户 ID
    pub friend_id: String,                      // 好友 ID
    pub nickname: Option<String>,               // 好友昵称
    pub remark: Option<String>,                 // 好友备注
    pub source_type: FriendSourceType,          // 好友来源类型
    pub friend_status: FriendStatus,            // 好友状态
    pub created_at: i64,                        // 创建时间
}
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub enum FriendStatus {
    #[default]
    Pending = 0, // 待验证
    Accepted = 1, // 已添加
    Deleted = 2, // 已删除
}
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub enum FriendSourceType{
    #[default]
    User = 0, // 用户主动添加
    Group = 1, // 群聊添加
    System = 2, // 系统推荐或其他方式添加
}