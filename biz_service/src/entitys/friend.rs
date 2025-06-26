use crate::protocol::protocol::{EventStatus, FriendEventType, FriendSourceType};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub struct FriendInfo {
    pub id: String,                             // 用户 ID
    pub agent_id: String,                       // 代理 ID
    pub uid: String,                            // 用户 ID
    pub friend_id: String,                      // 好友 ID
    pub nickname: Option<String>,               // 好友昵称
    pub remark: Option<String>,                 // 好友备注
    pub source_type: FriendSourceType,          // 好友来源类型
    pub created_at: i64,                        // 创建时间
    pub is_blocked: bool,                       // 是否已拉黑好友（uid 拉黑 friend_id）
}

/// 好友事件记录实体，用于数据库存储好友行为的详细信息。
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FriendEvent {
    /// 数据库主键；MongoDB 可为 ObjectId，关系型数据库可用 bigint 自增
    pub id: Option<String>,

    /// 所属租户 ID（多租户系统中区分不同业务方）
    pub agent_id: String,

    /// 操作发起方用户 ID（如发起请求/删除/拉黑的用户）
    pub from_user_id: String,

    /// 操作接收方用户 ID（如接收请求、被删除或被拉黑的用户）
    pub to_user_id: String,

    /// 操作类型（加好友、拉黑等）
    pub event_type: FriendEventType,

    /// 附加消息（例如加好友时的备注信息）
    pub message: Option<String>,

    /// 当前事件状态（如是否已处理、是否已撤销）
    pub status: EventStatus,

    /// 事件创建时间，单位为毫秒时间戳（Unix Epoch 毫秒）
    pub created_at: u64,

    /// 事件最后更新时间，单位为毫秒时间戳（如状态变化、备注更新等）
    pub updated_at: u64,
}

