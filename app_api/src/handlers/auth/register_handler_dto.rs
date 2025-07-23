use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::{Validate, ValidateEmail, ValidationError};

#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct RegisterRequest {
    /// 用户名（3-20位，字母数字下划线）
    #[validate(custom(function = "validate_username"))]
    pub username: String,

    /// 密码（至少8位，含字母和数字）
    #[validate(length(min = 8, message = "密码至少8位"))]
    #[validate(custom(function = "validate_password"))]
    pub password: String,

    /// 注册类型：1=Phone，2=Email，3=NFT
    pub reg_type: u8,

    /// 目标值：手机号 / 邮箱 / NFT 地址
    ///
    #[validate(custom(function = "validate_target"))]
    pub target: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RegisterResponse {
    pub uid: String,
}

// 邮箱和手机号校验
fn validate_target(value: &str) -> Result<(), ValidationError> {
    if value.contains('@') {
        // 假设为邮箱
        if value.validate_email() { Ok(()) } else { Err(ValidationError::new("邮箱格式无效")) }
    } else {
        // 假设为手机号，匹配国际电话格式 +86188xxx 或 001-xxx
        let phone_re = regex::Regex::new(r"^\+?[0-9]{7,20}$").unwrap();
        if phone_re.is_match(value) { Ok(()) } else { Err(ValidationError::new("国际电话格式错误")) }
    }
}
// 密码至少包含字母和数字
fn validate_password(pwd: &str) -> Result<(), ValidationError> {
    let has_letter = pwd.chars().any(|c| c.is_ascii_alphabetic());
    let has_digit = pwd.chars().any(|c| c.is_ascii_digit());

    if has_letter && has_digit { Ok(()) } else { Err(ValidationError::new("密码必须包含字母和数字")) }
}
fn validate_username(username: &str) -> Result<(), ValidationError> {
    return common::util::validate::validate_username(username);
}
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct RegisterVerifyRequest {
    /// 用户名（手机号或邮箱）
    #[validate(length(min = 8, message = "用户名太短"))]
    pub username: String,

    /// 登录密码
    #[validate(length(min = 8, message = "密码至少8位"))]
    #[validate(custom(function = "validate_password"))]
    pub password: String,

    /// 验证码
    #[validate(length(min = 6, message = "验证码格式错误"))]
    pub code: String,

    /// 验证码 Redis ID（服务端注册返回的 reg_id）
    #[validate(length(min = 8, message = "注册 ID 无效"))]
    pub reg_id: String,

    /// 注册方式：1=Phone，2=Email，3=NFT
    pub reg_type: u8,
}

/// 密码强度校验
fn validate_password_strength(pwd: &str) -> Result<(), ValidationError> {
    let has_letter = pwd.chars().any(|c| c.is_ascii_alphabetic());
    let has_digit = pwd.chars().any(|c| c.is_ascii_digit());
    if has_letter && has_digit { Ok(()) } else { Err(ValidationError::new("密码必须包含字母和数字")) }
}
