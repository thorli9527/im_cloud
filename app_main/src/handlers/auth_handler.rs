use crate::result::result;
use actix_web::{Responder, post, web};
use biz_service::biz_service::user_service::UserService;
use common::errors::AppError;
use common::repository_util::Repository;
use r#macro::QueryFilter;
use mongodb::bson;
use serde::{Deserialize, Serialize};
use std::option::Option;
use utoipa::ToSchema;
use validator::Validate;

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
    pub user_id: String,
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
pub async fn auth_login(// dto: web::Json<LoginInfoDto>,
    // session: Session,
    // state: web::Data<AppState>,
) -> Result<impl Responder, AppError> {
    let service = UserService::get();
    let x = service.dao.query_all().await.unwrap();

    if (1 == 1) {
        return Ok(result());
    }
    //
    // match &dto.validate() {
    //     Ok(_) => {
    //         let user_info_opt = user_service.dao.find_one(dto.to_query_doc()).await?;
    //         let user_info = match user_info_opt {
    //             Some(u) => u,
    //             None => {
    //                 if 1==1{
    //                     let mut new_user = UserInfo::default();
    //                     new_user.user_name = dto.user_name.as_ref().unwrap().to_string();
    //                     new_user.password = build_md5_with_key(&state.config.sys.md5_key, dto.password.as_ref().unwrap());
    //                     new_user.is_admin=true;
    //                     new_user.status=true;
    //                     new_user.create_time=1;
    //                     user_service.dao.insert(&new_user);
    //                 }
    //
    //                 return Ok(web::Json(result_error_msg("user.or.password.error")));
    //             }
    //         };
    //
    //         let password = build_md5_with_key(&state.config.sys.md5_key, dto.password.as_ref().unwrap());
    //         if password!=user_info.password{
    //             return Ok(web::Json(result_error_msg("user.or.password.error")));
    //         }
    //         let string = &state.config.sys.md5_key;
    //         let token = build_id();
    //         let login_session = LoginSessionDto { user_id: user_info.id.to_string(), user_name: user_info.user_name.clone(), token };
    //         session.insert("user_token", login_session.clone()).unwrap();
    //         return Ok(web::Json(result_data(login_session.token.clone())));
    //     }
    //     Err(e) => {
    //         return Ok(web::Json(result_error_msg(e.to_string().as_str())));
    //     }
    // }
    Ok(result())
}
#[utoipa::path(
    post,
    path = "/auth/info",
    request_body = LoginInfoDto,
    responses(
      (status = 200, description = "Hello response")
    )
)]
pub async fn auth_info() -> Result<LoginSessionDto, AppError> {
    // let result = session.get::<LoginSessionDto>("user_token")?;
    // if (result.is_some()) {
    //     return Ok(result.unwrap());
    // }
    return Err(AppError::Unauthorized("auth_error".to_string()));
}
#[utoipa::path(
    post,
    path = "/auth/logout",
    request_body = LoginInfoDto,
    responses(
      (status = 200, description = "Hello response")
    )
)]
#[post("/auth/logout")]
async fn auth_logout() -> Result<impl Responder, AppError> {
    Ok(result())
}
