use crate::handlers::auth::register_handler_dto::{RegisterRequest, RegisterResponse, RegisterVerifyRequest};
use crate::result::ApiResponse;
use actix_web::web::ServiceConfig;
use actix_web::{post, web, Responder};
use biz_service::manager::user_manager_auth::{UserManagerAuth, UserManagerAuthOpt, UserRegType};
use common::errors::AppError;
use log::error;
use serde_json::json;
use validator::Validate;

pub fn configure(cfg: &mut ServiceConfig) {
    cfg.service(auth_register);
    cfg.service(auth_register_verify);
}
#[utoipa::path(
    post,
    path = "/auth/register",
    tag = "Auth",
    request_body = RegisterRequest,
    responses(
        (status = 200, description = "注册成功", body = RegisterResponse),
        (status = 400, description = "参数格式错误"),
        (status = 500, description = "服务内部错误")
    )
)]
#[post("/auth/register")]
pub async fn auth_register(payload: web::Json<RegisterRequest>) -> Result<impl Responder, AppError> {
    // 参数校验
    if let Err(errs) = payload.validate() {
        let msg = format!("validate.error, {}", errs.to_string());
        return Ok(ApiResponse::json_error(400, msg));
    }

    let reg_type = match payload.reg_type {
        1 => UserRegType::Phone,
        2 => UserRegType::Email,
        3 => UserRegType::Nft,
        _ => return Ok(ApiResponse::json_error(400, "data.error")),
    };

    let user_manager = UserManagerAuth::get();

    match user_manager.register(&payload.username, &payload.password, &reg_type, &payload.target).await {
        Ok(reg_id) => {
            let body = json!({
                "regId": reg_id.to_string(),
            });
            return Ok(ApiResponse::json(body));
        }
        Err(e) => {
            log::error!("{}", e);
            return Ok(ApiResponse::json_error(400, "system.error"));
        }
    }
}
#[utoipa::path(
    post,
    path = "/auth/register/verify_code",
    tag = "Auth",
    request_body = RegisterVerifyRequest,
    responses(
        (status = 200, description = "注册成功", body = RegisterResponse),
        (status = 400, description = "参数错误或验证码无效"),
        (status = 500, description = "服务内部错误")
    )
)]
#[post("/auth/register/verify_code")]
pub async fn auth_register_verify(req: web::Json<RegisterVerifyRequest>) -> Result<impl Responder, AppError> {
    if let Err(errs) = req.validate() {
        error!("RegisterVerifyRequest validation failed: {}", errs.to_string());
        return Ok(ApiResponse::json_error(400, "system.error"));
    }

    let reg_type = match req.reg_type {
        1 => UserRegType::Phone,
        2 => UserRegType::Email,
        3 => UserRegType::Nft,
        _ => return Ok(ApiResponse::json_error(400, "invalid.data")),
    };

    let user_manager = UserManagerAuth::get();

    return match user_manager.register_verify_code(&req.username, &req.password, &req.reg_id, &req.code, &reg_type).await {
        Ok(uid) => {
            let body = json!({
                "uid": uid.to_string(),
            });
            Ok(ApiResponse::json(body))
        }
        Err(e) => {
            error!("Rregister_verify_code.error: {}", e.to_string());
            Ok(ApiResponse::json_error(500, "invalid.data"))
        }
    };
}
