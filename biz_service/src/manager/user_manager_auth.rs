#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(i32)]
pub enum UserAuthOpt {
    Register,
    Login,
    Logout,
    ChangePassword,
    ResetPassword,
}

#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(i32)]
pub enum UserRegType {
    Phone = 1,
    Email = 2,
    Nft = 3,
}
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(i32)]
pub enum ResetPasswordType {
    Phone = 1,
    Email = 2,
}

use crate::protocol::msg::auth::{AuthType, DeviceType};
use async_trait::async_trait;
use common::UserId;
use once_cell::sync::OnceCell;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct UserManagerAuth {}
impl UserManagerAuth {
    /// 构造新的 UserManagerAuth 实例
    ///
    /// # 参数
    /// - `pool`: Redis 连接池
    pub fn new() -> Self {
        let manager = Self {};
        return manager;
    }
    pub fn init() {
        let instance = UserManagerAuth::new();
        INSTANCE.set(Arc::new(instance)).expect("INSTANCE already initialized");
    }

    /// 获取全局实例（未初始化会 panic）
    pub fn get() -> Arc<Self> {
        INSTANCE.get().expect("UserManager is not initialized").clone()
    }
}

static INSTANCE: OnceCell<Arc<UserManagerAuth>> = OnceCell::new();

#[async_trait]
pub trait UserManagerAuthOpt: Send + Sync {
    async fn login_by_type(&self, password: &str, reg_type: &UserRegType, target: &str, device_type: &DeviceType) -> anyhow::Result<String>;
    /// 登录用户，将用户标记为在线，并进行必要的缓存更新和事件通知
    async fn login(
        &self,
        message_id: &u64,
        auth_type: &AuthType,
        auth_content: &str,
        password: &str,
        device_type: &DeviceType,
    ) -> anyhow::Result<String>;

    async fn logout(&self, message_id: &u64, user_id: &UserId, device_type: &DeviceType) -> anyhow::Result<()>;
    /// 注册新用户
    async fn register(
        &self,
        password: &str,
        reg_type: &UserRegType, // 注册方式
        target: &str,           // 手机号或邮箱
    ) -> anyhow::Result<String>; // 返回用户ID或Token

    async fn register_verify_code(&self, password: &str, reg_id: &str, code: &str, reg_type: &UserRegType) -> anyhow::Result<String>;

    /// 修改登录密码
    async fn change_password(&self, token: &str, old_password: &str, new_password: &str) -> anyhow::Result<()>;

    /// 构建验证码（发送验证码）
    async fn reset_password_build_code(&self, reset_type: &ResetPasswordType, user_name: &str) -> anyhow::Result<()>;

    /// 验证验证码并重置密码
    async fn reset_password_verify_code(&self, reset_type: &ResetPasswordType, user_name: &str, code: &str, new_password: &str)
    -> anyhow::Result<()>;
}
