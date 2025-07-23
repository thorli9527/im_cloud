use crate::handlers::auth::register_handler_dto::{
    RegisterRequest, RegisterResponse, RegisterVerifyRequest,
};
use actix_web::web::{Json, ServiceConfig};
use actix_web::{post, web, HttpResponse, Responder};
use biz_service::manager::user_manager_auth::{UserManagerAuth, UserManagerAuthOpt, UserRegType};
use utoipa::ToSchema;
use validator::Validate;

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
pub async fn auth_register(
    Json(payload): Json<RegisterRequest>,
) -> Result<Json<RegisterResponse>, (axum::http::StatusCode, String)> {
    // 参数校验
    if let Err(errs) = payload.validate() {
        return Err((axum::http::StatusCode::BAD_REQUEST, format!("❌ 参数校验失败: {}", errs)));
    }

    let reg_type = match payload.reg_type {
        1 => UserRegType::Phone,
        2 => UserRegType::Email,
        3 => UserRegType::Nft,
        _ => return Err((axum::http::StatusCode::BAD_REQUEST, "invalid.data".to_string())),
    };

    let user_manager = UserManagerAuth::get();

    match user_manager
        .register(&payload.username, &payload.password, &reg_type, &payload.target)
        .await
    {
        Ok(uid) => Ok(Json(RegisterResponse { user_id: uid })),
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("注册失败: {}", e))),
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
pub async fn auth_register_verify(req: web::Json<RegisterVerifyRequest>) -> impl Responder {
    if let Err(errs) = req.validate() {
        return HttpResponse::BadRequest().body(format!("❌ 参数校验失败: {}", errs));
    }

    let reg_type = match req.reg_type {
        1 => UserRegType::Phone,
        2 => UserRegType::Email,
        3 => UserRegType::Nft,
        _ => return HttpResponse::BadRequest().body("❌ 注册类型不合法"),
    };

    let user_manager = UserManagerAuth::get();
    match user_manager
        .register_verify_code(&req.username, &req.password, &req.reg_id, &req.code, &reg_type)
        .await
    {
        Ok(uid) => HttpResponse::Ok().json(RegisterResponse { user_id: uid }),
        Err(e) => HttpResponse::BadRequest().body(format!("注册失败: {}", e)),
    }
}

pub(crate) fn configure(cfg: &mut ServiceConfig) {
    cfg.service(auth_register);
    cfg.service(auth_register_verify);
}
