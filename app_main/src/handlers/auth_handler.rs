use crate::result::result;
use actix_web::{HttpResponse, Responder, post, web};
use biz_core::service::user_role_service::UserRoleService;
use biz_core::service::user_service::UserService;
use common::errors::AppError;
use common::repository_util::Repository;
use mongo_macro::QueryFilter;
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};
use std::option::Option;
use utoipa::ToSchema;
use validator::Validate;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(user_login);
    cfg.service(auth_logout);
}

#[derive(QueryFilter, Serialize, Deserialize, Debug, Validate, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LoginInfoDto {
    #[query(eq)]
    #[validate(length(min = 5, message = "name.too.short"))]
    pub user_name: Option<String>,
    #[validate(length(min = 5, message = "password.too.short"))]
    pub password: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LoginSessionDto {
    pub token: String,
    pub user_name: String,
}

#[utoipa::path(
    post,
    path = "/auth/logout",
    request_body = LoginInfoDto,
    responses(
        (status = 200, description = "Hello response", body = String)
    )
)]
#[post("/auth/logout")]
async fn auth_logout(dto: web::Json<LoginSessionDto>) -> Result<impl Responder, AppError> {
    let user_service = UserService::get();
    user_service.login_out(&dto.user_name).await?;
    Ok(result())
}

/// 登录请求DTO
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct LoginRequest {
    user_name: String,
    password: String,
}

/// 登录响应DTO
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct LoginResponse {
    token: String,
    roles: Vec<String>,
}

#[utoipa::path(
    post,
    path = "/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "登录成功", body = LoginResponse),
        (status = 401, description = "用户名或密码错误"),
        (status = 500, description = "登录过程异常")
    )
)]
// 登录处理函数
#[post("/auth/login")]
async fn user_login(req: web::Json<LoginRequest>) -> impl Responder {
    let username = &req.user_name;
    let password = &req.password;
    // 调用 UserService 验证用户
    match UserService::get().build_login(&username.clone(), password).await {
        Ok((token, user)) => {
            // 获取用户角色列表
            let roles =
                UserRoleService::get().get_roles_by_user(&user.id).await.unwrap_or_default();
            let role_codes: Vec<String> = roles.iter().map(|r| r.code.clone()).collect();

            // 构建登录响应
            let response = LoginResponse { token, roles: role_codes };
            HttpResponse::Ok().json(response)
        }
        Err(e) => {
            eprintln!("Login error: {}", e);
            HttpResponse::Unauthorized().body("Invalid username or password")
        }
    }
}
