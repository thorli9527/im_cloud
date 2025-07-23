use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

/// 密码重置方式
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
pub enum ResetPasswordType {
    #[serde(rename = "phone")]
    Phone = 1,
    #[serde(rename = "email")]
    Email = 2,
}
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ResetPasswordSendRequest {
    /// 用户名（手机号或邮箱）
    pub user_name: String,

    /// 重置方式：1=Phone，2=Email
    pub reset_type: u8,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ResetPasswordVerifyRequest {
    /// 用户名
    pub user_name: String,

    /// 验证码
    #[validate(length(min = 6, message = "验证码格式不正确"))]
    pub code: String,

    /// 新密码（至少 8 位，包含字母和数字）
    #[validate(length(min = 8, message = "密码至少 8 位"))]
    #[validate(custom(function = "validate_password"))]
    pub new_password: String,

    /// 重置方式
    pub reset_type: u8,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ResetPasswordResponse {
    pub message: String,
}

fn validate_password(value: &str) -> Result<(), validator::ValidationError> {
    return common::util::validate::validate_password(value);
}
