use crate::result::{AppState, ResultResponse};
use actix_session::Session;
use actix_web::{post, web, Responder};
use actix_web::web::Data;
use biz_service::biz_services::user_service::UserService;
use common::errors::AppError;
use common::repository_util::Repository;
use common::util::common_utils::{build_id, build_md5, build_md5_with_key};
use mongodb::bson;
use r#macro::QueryFilter;
use serde::{Deserialize, Serialize};
use utoipa::openapi::security::Password;
use utoipa::ToSchema;
use validator::Validate;
use biz_service::entitys::user_entity::UserInfo;
use crate::handlers::common_handler::status;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(auth_login);
    cfg.service(auth_logout);
}

#[derive(QueryFilter, Serialize, Deserialize, Debug, Validate, ToSchema,Clone)]
#[serde(rename_all = "camelCase")]
pub struct LoginInfoDto {
    #[query(eq)]
    #[validate(length(min = 5, message = "用户名太短"))]
    pub user_name: Option<String>,
    #[validate(length(min = 5, message = "密码太短"))]
    pub password: Option<String>,
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
pub async fn auth_login(dto: web::Json<LoginInfoDto>,
                        session: Session,
                        state: web::Data<AppState>,
                        user_service: web::Data<UserService>,
) -> Result<impl Responder, AppError> {
    match &dto.validate() {
        Ok(_) => {
            let user_info = user_service.dao.find_one(dto.to_query_doc()).await?;
            if user_info.is_none() {
                let string = &state.config.sys.md5_key;
                let mut new_user=UserInfo::default();
                new_user.user_name= dto.user_name.as_ref().unwrap().to_string();
                new_user.password=build_md5_with_key(&state.config.sys.md5_key,dto.password.as_ref().unwrap());
                user_service.dao.insert(&new_user);
                return Ok(web::Json(ResultResponse::<String>::err("用户名或密码错误")));
            }
            let token = build_id();
            session.insert(&token, user_info.unwrap().user_name);
            return Ok(web::Json(ResultResponse::err("登录成功")));
        }
        Err(e) => {
            return Ok(web::Json(ResultResponse::<String>::err(e.to_string())))
        }
    }
}
#[post("/auth/logout")]
async fn auth_logout(session: Session) -> Result<impl Responder, AppError> {
    Ok(web::Json(ResultResponse::<String>::err("用户名或密码错误")))
}
