use crate::result::{result, result_data, result_error_msg, ResultResponse};
use actix_session::Session;
use actix_web::{post, web, Responder};
use biz_service::biz_services::user_service::UserService;
use common::errors::AppError;
use common::repository_util::Repository;
use common::util::common_utils::build_id;
use mongodb::bson;
use r#macro::QueryFilter;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(auth_login);
    cfg.service(auth_logout);
}

#[derive(QueryFilter, Serialize, Deserialize, Debug,Validate,ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LoginInfoDto {
    #[query(eq)]
    #[validate(length(min = 5, message = "用户名太短"))]
    pub user_name: Option<String>,
}
#[derive(Debug, Serialize, Deserialize,ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LoginResult{
    login_status:bool,
    token:String,
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
                    user_info: web::Data<UserService>,
) -> Result<impl Responder, AppError> {
    match dto.validate() {
        Ok(_) => {
            let user_info = user_info.dao.find_one(dto.to_query_doc()).await?;
            if user_info.is_none(){
                return Ok(web::Json(ResultResponse::<String>::err("用户名或密码错误")))
            }
            let token = build_id();
            session.insert(&token,user_info.unwrap().user_name);
             return Ok(web::Json(ResultResponse::err("登录成功")))
        },
        Err(e) => {
            return Ok(web::Json(ResultResponse::<String>::err(e.to_string())))
        },
    }
}
#[post("/auth/logout")]
async fn auth_logout(session: Session) -> Result<impl Responder, AppError> {
    Ok(web::Json(ResultResponse::<String>::err("用户名或密码错误")))
}
