use actix_web::{HttpResponse, ResponseError};
use deadpool_redis::redis::RedisError;
use deadpool_redis::PoolError;
use log::error;
use mongodb::error::Error as MongoError;
use serde::Serialize;
use std::io;
use thiserror::Error;
/// HTTP 错误响应结构
#[derive(Serialize)]
struct ErrorResponse {
    code: u16,
    message: String,
}

/// 应用错误类型
#[derive(Debug, Error)]
pub enum AppError {
    // ==== 常规业务错误 ====
    #[error("Resource not found")]
    NotFound,

    #[error("Bad request: {0}")]
    Validation(String),

    #[error("Unauthorized access")]
    Unauthorized(String),
    #[error("biz error: {0}")]
    BizError(String),

    #[error("Forbidden access")]
    Forbidden,

    #[error("Conflict: resource already exists")]
    Conflict,

    #[error("Too many requests")]
    RateLimited,

    #[error("File upload failed: {0}")]
    FileUpload(String),

    #[error("External API call failed: {0}")]
    ExternalApi(String),
    #[error("Redis pool error: {0}")]
    RedisPoolError(#[from] PoolError),
    // ==== 系统错误 ====
    #[error("MongoDB error: {0}")]
    Mongo(#[from] MongoError),
    #[error("Redis error: {0}")]
    Redis(#[from] RedisError),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("socket: {0}")]
    SocketError(String),
    #[error("Internal server error")]
    Internal(String),
    #[error("Conversion error")]
    ConversionError,
}
impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        AppError::Internal(e.to_string())
    }
}
impl From<validator::ValidationErrors> for AppError {
    fn from(e: validator::ValidationErrors) -> Self {
        AppError::BizError(format!("参数验证失败: {}", e))
    }
}
impl ResponseError for AppError {
    fn error_response(&self) -> HttpResponse {
        let (status, msg) = match self {
            AppError::NotFound => (actix_web::http::StatusCode::NOT_FOUND, self.to_string()),
            AppError::ConversionError => (actix_web::http::StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            AppError::Validation(_) => (actix_web::http::StatusCode::BAD_REQUEST, self.to_string()),
            AppError::Unauthorized(msg) => (actix_web::http::StatusCode::UNAUTHORIZED, msg.to_string()),
            AppError::Forbidden => (actix_web::http::StatusCode::FORBIDDEN, self.to_string()),
            AppError::Conflict => (actix_web::http::StatusCode::CONFLICT, self.to_string()),
            AppError::RateLimited => (actix_web::http::StatusCode::TOO_MANY_REQUESTS, self.to_string()),
            AppError::FileUpload(_) | AppError::ExternalApi(_) => (actix_web::http::StatusCode::BAD_GATEWAY, self.to_string()),
            AppError::Mongo(e) => {
                error!("{:?}", e);
                (actix_web::http::StatusCode::INTERNAL_SERVER_ERROR, "Service error".to_string())
            }
            AppError::Redis(e) => {
                error!("{:?}", e);
                (actix_web::http::StatusCode::INTERNAL_SERVER_ERROR, "Service error".to_string())
            }
            AppError::Io(e) => {
                error!("{:?}", e);
                (actix_web::http::StatusCode::INTERNAL_SERVER_ERROR, "Service error".to_string())
            }
            AppError::Json(e) => {
                error!("{:?}", e);
                (actix_web::http::StatusCode::INTERNAL_SERVER_ERROR, "Service error".to_string())
            }
            AppError::Internal(e) => {
                error!("{:?}", e);
                (actix_web::http::StatusCode::INTERNAL_SERVER_ERROR, "Service error".to_string())
            }
            AppError::RedisPoolError(e) => {
                error!("{:?}", e);
                (actix_web::http::StatusCode::INTERNAL_SERVER_ERROR, "Service error".to_string())
            }
            AppError::BizError(e) => {
                error!("{:?}", e);
                (actix_web::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            }
            e => {
                error!("{:?}", e);
                (actix_web::http::StatusCode::INTERNAL_SERVER_ERROR, "Service error".to_string())
            }
        };

        HttpResponse::build(status).json(ErrorResponse { code: status.as_u16(), message: msg })
    }
}
