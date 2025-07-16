use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use common::index_trait::MongoIndexModelProvider;
use mongo_macro::MongoIndexModelProvider as MongoDeriveMongoIndex;
/// 角色权限关联结构体，表示某个角色拥有的权限项
/// 角色与权限关联表，用于表示角色拥有哪些权限点（多对多关系）
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema, MongoDeriveMongoIndex)]
#[mongo_index(fields["role_id", "permission_id"], unique)]
pub struct RolePermissionEntity {
    /// 关联记录 ID（唯一，可用 UUID 或 ObjectId 字符串）
    pub id: String,
    /// 角色 ID
    pub role_id: String,
    /// 权限 ID
    pub permission_id: String,
    /// 创建时间
    pub create_time: u64,
}