use crate::result::{result, AppState};
use actix_web::{get, web, Responder};
use mongodb::bson::DateTime;
use biz_service::biz_services::user_service;
use biz_service::biz_services::user_service::UserService;
use biz_service::entitys::user_entity::UserInfo;

use common::errors::AppError;
use common::repository_util::Repository;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(user_list);
}

#[get("/status")]
pub async fn user_list(user_service:web::Data<UserService>) -> Result<impl Responder, AppError> {
    Ok(web::Json(result()))
}
