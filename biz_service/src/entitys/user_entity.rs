use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// 用户信息结构体，用于存储系统用户账号、权限和身份信息
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub struct UserInfoEntity {
    /// 用户唯一 ID（通常是 MongoDB 的 ObjectId，以字符串形式保存）
    pub id: String,
    /// 用户名（用于登录）
    pub user_name: String,
    /// 密码（应加密存储，例如 bcrypt 或 argon2）
    pub password: String,
    /// 用户状态（true 表示启用，false 表示禁用/冻结）
    pub status: bool,
    /// 是否为管理员（true 表示拥有系统管理权限）
    pub is_admin: bool,
    /// 创建时间（Unix 秒时间戳）
    pub create_time: i64,
    /// 最后更新时间（Unix 秒时间戳）
    pub update_time: i64,
}
