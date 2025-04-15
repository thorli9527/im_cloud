use std::process::id;
use crate::handlers::common_handler::status;
use crate::result::{AppState, ApiResponse, result, result_error_msg, result_data};
use actix_session::Session;
use actix_web::web::Data;
use actix_web::{Responder, post, web};
use biz_service::biz_services::menu_service::MenuService;
use biz_service::biz_services::role_menu_service::RoleMenuRelService;
use biz_service::biz_services::user_role_service::UserRoleService;
use biz_service::biz_services::user_service::{MenuDto, UserService};
use biz_service::entitys::user_entity::UserInfo;
use common::errors::AppError;
use common::repository_util::Repository;
use common::util::common_utils::{build_id, build_md5, build_md5_with_key};
use r#macro::QueryFilter;
use mongodb::bson;
use mongodb::bson::DateTime;
use mongodb::change_stream::session;
use serde::{Deserialize, Serialize};
use std::option::Option;
use utoipa::ToSchema;
use utoipa::openapi::security::Password;
use validator::Validate;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(auth_login);
    cfg.service(auth_logout);
}

#[derive(QueryFilter, Serialize, Deserialize, Debug, Validate, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LoginInfoDto {
    #[query(eq)]
    #[validate(length(min = 5, message = "用户名太短"))]
    pub user_name: Option<String>,
    #[validate(length(min = 5, message = "密码太短"))]
    pub password: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default,ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LoginSessionDto {
    pub token: String,
    pub user_id: String,
    pub user_name: String,
    pub user_menu: Vec<MenuDto>,
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
pub async fn auth_login(
    dto: web::Json<LoginInfoDto>,
    session: Session,
    state: web::Data<AppState>,
    user_service: web::Data<UserService>,
    user_role_service: Data<UserRoleService>,
    role_menu_service: Data<RoleMenuRelService>,
    menu_service: Data<MenuService>,
) -> Result<impl Responder, AppError> {
    match &dto.validate() {
        Ok(_) => {
            let user_info_opt = user_service.dao.find_one(dto.to_query_doc()).await?;
            let user_info = match user_info_opt {
                Some(u) => u,
                None => {
                    if 1==1{
                        let mut new_user = UserInfo::default();
                        new_user.user_name = dto.user_name.as_ref().unwrap().to_string();
                        new_user.password = build_md5_with_key(&state.config.sys.md5_key, dto.password.as_ref().unwrap());
                        new_user.is_admin=true;
                        new_user.status=true;
                        new_user.create_time=1;
                        user_service.dao.insert(&new_user);
                    }

                    return Ok(web::Json(result_error_msg("user.or.password.error")));
                }
            };

            let password = build_md5_with_key(&state.config.sys.md5_key, dto.password.as_ref().unwrap());
            if password!=user_info.password{
                return Ok(web::Json(result_error_msg("user.or.password.error")));
            }
            let string = &state.config.sys.md5_key;
            let user_menu = user_service
                .load_user_menu(user_info.id.to_string(), user_role_service,role_menu_service, menu_service)
                .await?;
            let token = build_id();
            let login_session = LoginSessionDto { user_id: user_info.id.to_string(), user_name: user_info.user_name.clone(), user_menu, token };
            session.insert("user_token", login_session.clone()).unwrap();
            return Ok(web::Json(result_data(login_session.token.clone())));
        }
        Err(e) => {
            return Ok(web::Json(result_error_msg(e.to_string().as_str())));
        }
    }
}
#[utoipa::path(
    post,
    path = "/auth/info",
    request_body = LoginInfoDto,
    responses(
      (status = 200, description = "Hello response", body = ApiResponse<LoginSessionDto>)
    )
)]
pub async fn auth_info(session: Session) -> Result<LoginSessionDto, AppError> {
    let result = session.get::<LoginSessionDto>("user_token")?;
    if (result.is_some()) {
        return Ok(result.unwrap());
    }
    return Err(AppError::Unauthorized);
}
#[utoipa::path(
    post,
    path = "/auth/logout",
    request_body = LoginInfoDto,
    responses(
      (status = 200, description = "Hello response", body = ApiResponse<LoginSessionDto>)
    )
)]
#[post("/auth/logout")]
async fn auth_logout(session: Session) -> Result<impl Responder, AppError> {
    return Ok(web::Json(result()));
}
