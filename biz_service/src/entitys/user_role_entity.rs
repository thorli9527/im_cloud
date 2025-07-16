use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use common::index_trait::MongoIndexModelProvider;
use mongo_macro::MongoIndexModelProvider as MongoDeriveMongoIndex;
/// 用户角色关联结构体，表示用户与角色之间的绑定关系
/// 用户与角色关联表（多对多关系）
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema, MongoDeriveMongoIndex)]
#[mongo_index(fields["user_id", "role_id"], unique)]
pub struct UserRoleEntity {
    /// 关联记录 ID（唯一）
    pub id: String,
    /// 用户 ID
    pub user_id: String,
    /// 角色 ID
    pub role_id: String,
    /// 创建时间
    pub create_time: u64,
}