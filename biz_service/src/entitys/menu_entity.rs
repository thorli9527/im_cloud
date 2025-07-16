use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use common::index_trait::MongoIndexModelProvider;
use mongo_macro::MongoIndexModelProvider as MongoDeriveMongoIndex;

/// 菜单结构，用于构建前端动态菜单树
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema, MongoDeriveMongoIndex)]
#[mongo_index(fields["path"], unique)]
pub struct MenuEntity {
    /// 菜单 ID（唯一字符串，例如 MongoDB ObjectId 字符串形式）
    pub id: String,
    /// 菜单标题
    pub title: String,
    /// 父级菜单 ID（根节点为 "" 或 None）
    pub parent_id: Option<String>,
    /// 路由路径
    pub path: String,
    /// 前端组件名
    pub component: Option<String>,
    /// 图标名称
    pub icon: Option<String>,
    /// 权限码（可选，用于控制该菜单的访问权限）
    pub permission: Option<String>,
    /// 是否隐藏菜单
    pub hidden: bool,
    /// 菜单排序（数值越小越靠前）
    pub order: i32,
    /// 创建时间（Unix 时间戳，秒）
    pub create_time: u64,
}