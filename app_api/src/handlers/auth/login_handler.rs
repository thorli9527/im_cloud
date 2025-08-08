use actix_web::{post, web, web::ServiceConfig, HttpResponse, Responder};
use biz_core::manager::user_manager_auth::{UserManagerAuth, UserManagerAuthOpt, UserRegType};
use biz_core::protocol::msg::auth::DeviceType;
use common::errors::AppError;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub fn configure(cfg: &mut ServiceConfig) {
    cfg.service(auth_login);
}

#[derive(Deserialize, Serialize, ToSchema, Debug)]
pub struct LoginReq {
    #[schema(example = "phone")]
    login_type: UserRegType,
    #[schema(example = "123456")]
    password: String,
    #[schema(example = "+8613888888888")]
    target: String,
    #[schema(example = "Android")]
    device_type: DeviceType,
}

#[derive(Deserialize, Serialize, ToSchema, Debug)]
pub struct LoginResp {
    #[schema(example = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...")]
    token: String,
}

/// 用户登录
///
/// 用户可以使用手机号、邮箱、用户名等方式登录
#[utoipa::path(
    post,
    path = "/auth/login",
    request_body = LoginReq,
    responses(
        (status = 200, description = "登录成功", body = LoginResp),
        (status = 401, description = "登录失败，用户名或密码错误"),
        (status = 500, description = "服务器错误"),
    ),
    tag = "auth"
)]
#[post("/auth/login")]
pub async fn auth_login(dto: web::Json<LoginReq>) -> Result<impl Responder, AppError> {
    let user_manager_auth = UserManagerAuth::get();
    let token_result = user_manager_auth.login_by_type(&dto.password, &dto.login_type, &dto.target, &dto.device_type).await;

    match token_result {
        Ok(token) => {
            let resp = LoginResp {
                token,
            };
            Ok(HttpResponse::Ok().json(resp))
        }
        Err(e) => {
            eprintln!("Login failed: {:?}", e);
            Ok(HttpResponse::Unauthorized().body("Login failed: Invalid credentials")) // 401
        }
    }
}
