use crate::protocol::msg::friend::FriendSourceType;
use common::index_trait::MongoIndexModelProvider;
use mongo_macro::MongoIndexModelProvider as MongoDeriveMongoIndex;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema, MongoDeriveMongoIndex)]
#[mongo_index(fields["uid", "friend_id"], unique)]
pub struct FriendEntity {
    pub id: String,                    // 用户 ID
    pub uid: String,                   // 用户 ID
    pub friend_id: String,             // 好友 ID
    pub nickname: String,      // 好友昵称
    pub remark: Option<String>,        // 好友备注
    pub source_type: FriendSourceType, // 好友来源类型
    pub created_at: u64,               // 创建时间
    pub is_blocked: bool,              // 是否已拉黑好友（uid 拉黑 friend_id）
}
