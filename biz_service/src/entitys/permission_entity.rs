use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use common::index_trait::MongoIndexModelProvider;
use mongo_macro::MongoIndexModelProvider as MongoDeriveMongoIndex;
/// 权限点结构，用于表示系统中可控的操作或资源
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema, MongoDeriveMongoIndex)]
#[mongo_index(fields["code"], unique)]
pub struct PermissionEntity {
    /// 权限 ID（唯一）
    pub id: String,
    /// 权限名称（例如 "用户新增"）
    pub name: String,
    /// 权限码（例如 "user:add"，用于后端判断和前端控制按钮显示）
    pub code: String,
    /// 所属模块或分类（例如 "user", "system"）
    pub module: String,
    /// 描述信息
    pub description: Option<String>,
    /// 是否启用该权限
    pub enabled: bool,
    /// 创建时间（Unix 时间戳，秒）
    pub create_time: u64,
    /// 最后更新时间（Unix 时间戳，秒）
    pub update_time: u64,
}