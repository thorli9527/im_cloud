use crate::handlers::auth::reset_password_handler_dto::{
    ResetPasswordResponse, ResetPasswordSendRequest, ResetPasswordVerifyRequest,
};
use crate::result::result;
use actix_web::web::ServiceConfig;
use actix_web::{post, web, HttpResponse, Responder};
use biz_service::manager::user_manager_auth::{
    ResetPasswordType, UserManagerAuth, UserManagerAuthOpt,
};
use tracing::log;
use validator::Validate;

#[utoipa::path(
    post,
    path = "/auth/reset_password/send_code",
    tag = "Auth",
    request_body = ResetPasswordSendRequest,
    responses(
        (status = 200, description = "验证码已发送", body = ResetPasswordResponse),
        (status = 400, description = "参数错误")
    )
)]
#[post("/auth/reset_password/send_code")]
pub async fn auth_reset_password_send_code(
    req: web::Json<ResetPasswordSendRequest>,
) -> impl Responder {
    let reset_type = match req.reset_type {
        1 => ResetPasswordType::Phone,
        2 => ResetPasswordType::Email,
        _ => return HttpResponse::BadRequest().body("invalid.data"),
    };

    let user_manager = UserManagerAuth::get();
    match user_manager.reset_password_build_code(&reset_type, &req.user_name).await {
        Ok(_) => HttpResponse::Ok()
            .json(ResetPasswordResponse { message: "验证码已发送".to_string() }),
        Err(e) => {
            log::error!("重置密码发送验证码失败: {}", e);
            HttpResponse::BadRequest().body("system.error")
        }
    }
}

#[utoipa::path(
    post,
    path = "/auth/reset_password/verify_code",
    tag = "Auth",
    request_body = ResetPasswordVerifyRequest,
    responses(
        (status = 200, description = "密码已重置", body = ResetPasswordResponse),
        (status = 400, description = "验证码错误或参数不合法")
    )
)]
#[post("/auth/reset_password/verify_code")]
pub async fn auth_reset_password_verify_code(
    req: web::Json<ResetPasswordVerifyRequest>,
) -> impl Responder {
    if let Err(e) = req.validate() {
        return HttpResponse::BadRequest().body(format!("参数校验失败: {}", e));
    }

    let reset_type = match req.reset_type {
        1 => ResetPasswordType::Phone,
        2 => ResetPasswordType::Email,
        _ => return HttpResponse::BadRequest().body("invalid.data"),
    };

    let user_manager = UserManagerAuth::get();
    match user_manager
        .reset_password_verify_code(&reset_type, &req.user_name, &req.code, &req.new_password)
        .await
    {
        Ok(_) => Ok(result()),
        Err(e) => {
            log::error!("重置密码验证码失败: {}", e);
            HttpResponse::BadRequest().body("system.error")
        }
    }
}

pub(crate) fn configure(cfg: &mut ServiceConfig) {
    cfg.service(auth_reset_password_send_code);
    cfg.service(auth_reset_password_verify_code);
}
