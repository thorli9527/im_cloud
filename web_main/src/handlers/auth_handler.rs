use actix_session::Session;
use actix_web::{post, web, HttpResponse, Responder};
use actix_web::middleware::Identity;
use anyhow::anyhow;
use mongodb::change_stream::session;
use serde::{Deserialize, Serialize};
use utoipa::openapi::security::Password;
use biz_service::biz_services::user_service::UserService;
use biz_service::entitys::user_entity::UserInfo;
use common::errors::AppError;
use common::repository_util::Repository;
use r#macro::QueryFilter;
use mongodb::bson;
use crate::handlers::common_handler::status;
use crate::result::{result, result_data, result_error_msg, AppState};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(auth_login);
    cfg.service(auth_logout);
}

#[derive(QueryFilter, Serialize, Deserialize, Debug)]

struct LoginInfoDto {
    #[query(eq)]
    pub user_name: Option<String>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoginResult{
    login_status:bool,
    token:String,

}
#[post("/auth/login")]
async fn auth_login(login_info: web::Json<LoginInfoDto>,
                    app_state: web::Data<AppState>,
                    session: Session,
                    user_info: web::Data<UserService>,
) -> Result<impl Responder, AppError> {
    let user_info = user_info.dao.find_one(login_info.to_query_doc()).await?;
    if user_info.is_none(){
        let value = result_error_msg("username.or.password.error".to_string());
        return Ok(web::Json(result_data(value)))
    }
    // let id = user_id.into_inner().user_id;
    // session.insert("user_id", &id).map_err(|e|AppError::Internal(anyhow!("insert userSession error: {}",id)));
    // session.renew();
    Ok(web::Json(result_data("d")))
}
#[post("/auth/logout")]
async fn auth_logout(session: Session) -> Result<impl Responder, AppError> {
    Ok(web::Json(result()))
}
