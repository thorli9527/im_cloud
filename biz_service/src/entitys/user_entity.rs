use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use common::index_trait::MongoIndexModelProvider;
use mongo_macro::MongoIndexModelProvider as MongoDeriveMongoIndex;

/// 用户信息结构体，用于存储系统用户账号、权限和身份信息
/// 用户信息结构
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema, MongoDeriveMongoIndex)]
#[mongo_index(fields["user_name"], unique)]
pub struct UserInfoEntity {
    /// 用户唯一 ID（字符串形式）
    pub id: String,
    /// 用户名（用于登录）
    pub user_name: String,
    /// 加密后的密码（存储哈希，例如 bcrypt）
    pub password: String,
    /// 用户状态（true=启用, false=禁用/冻结）
    pub status: bool,
    /// 是否为管理员（true=超级管理员，拥有所有权限）
    pub is_admin: bool,
    /// 创建时间（Unix 时间戳，秒）
    pub create_time: u64,
    /// 最后更新时间（Unix 时间戳，秒）
    pub update_time: u64,
}