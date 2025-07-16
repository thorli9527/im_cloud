use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use common::index_trait::MongoIndexModelProvider;
use mongo_macro::MongoIndexModelProvider as MongoDeriveMongoIndex;
/// 角色信息结构体，用于定义角色权限集合
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema, MongoDeriveMongoIndex)]
#[mongo_index(fields["code"], unique)]
pub struct RoleEntity {
    /// 角色 ID（唯一）
    pub id: String,
    /// 角色名称（如 "管理员"、"编辑"、"普通用户" 等）
    pub name: String,
    /// 角色编码（如 "admin"、"editor"，用于程序判断）
    pub code: String,
    /// 描述信息
    pub description: Option<String>,
    /// 是否为系统预置角色（预置角色通常不允许删除）
    pub is_builtin: bool,
    /// 创建时间（Unix 时间戳，秒）
    pub create_time: u64,
    /// 最后更新时间（Unix 时间戳，秒）
    pub update_time: u64,
}
