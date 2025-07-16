use crate::result::{result, result_data, result_error_msg};
use actix_web::{Responder, post, web};
use biz_service::biz_service::user_service::UserService;
use common::errors::AppError;
use mongodb::bson;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::option::Option;
use utoipa::ToSchema;
use validator::Validate;
use mongo_macro::{ QueryFilter};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(auth_login);
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
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LoginResult {
    login_status: bool,
    token: String,
}

#[utoipa::path(
    post,
    path = "/auth/login",
    request_body = LoginInfoDto,
    responses(
        (status = 200, description = "Hello response", body = String)
    )
)]
#[post("/auth/login")]
pub async fn auth_login(dto: web::Json<LoginInfoDto>) -> Result<impl Responder, AppError> {
    if let Err(e) = dto.validate() {
        return Ok(web::Json(result_error_msg(&e.to_string())));
    }
    let user_name = &dto.user_name.clone().unwrap();
    let password = &dto.password.clone().unwrap();
    let user_service = UserService::get();
    let (token, user_info) = user_service.build_login(user_name, password).await?;
    let value = json!({"token":token,"user_name":user_info.user_name});
    Ok(web::Json(result_data(value)))
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
