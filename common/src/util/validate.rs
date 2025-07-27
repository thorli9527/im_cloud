use lazy_static::lazy_static;
use regex::Regex;
use validator::{ValidateEmail, ValidationError};

/// ✅ 密码强度校验：最少8位 + 字母 + 数字
pub fn validate_password(pwd: &str) -> Result<(), ValidationError> {
    if pwd.len() < 8 {
        return Err(ValidationError::new("密码至少8位"));
    }
    let has_letter = pwd.chars().any(|c| c.is_ascii_alphabetic());
    let has_digit = pwd.chars().any(|c| c.is_ascii_digit());

    if has_letter && has_digit { Ok(()) } else { Err(ValidationError::new("密码必须包含字母和数字")) }
}

/// ✅ 国际手机号校验（可接受 +86、+1、0086 格式）
pub fn validate_phone(phone: &str) -> Result<(), ValidationError> {
    lazy_static! {
        static ref PHONE_RE: Regex = Regex::new(r"^\+?[0-9]{7,20}$").unwrap();
    }

    if PHONE_RE.is_match(phone) { Ok(()) } else { Err(ValidationError::new("国际手机号格式错误")) }
}
pub fn validate_username(value: &str) -> Result<(), ValidationError> {
    let len = value.len();

    // 长度限制
    if len < 4 || len > 20 {
        return Err(ValidationError::new("user.name.over.len"));
    }

    // 仅允许字母、数字、下划线
    if !value.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(ValidationError::new("用户名仅允许字母、数字和下划线"));
    }

    // 黑名单（可选）
    let blacklist = ["admin", "root", "superuser"];
    if blacklist.contains(&value.to_lowercase().as_str()) {
        return Err(ValidationError::new("用户名已被保留"));
    }

    Ok(())
}

/// ✅ 邮箱格式校验（基于 validator 库）
pub fn validate_email_str(email: &str) -> Result<(), ValidationError> {
    if email.validate_email() { Ok(()) } else { Err(ValidationError::new("邮箱格式错误")) }
}
