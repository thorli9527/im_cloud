use crate::result::{result, AppState};
use actix_web::{get, web, Responder};
use mongodb::bson::DateTime;
use biz_service::biz_services::path_service::PathService;
use biz_service::biz_services::user_service;
use biz_service::biz_services::user_service::UserService;
use biz_service::entitys::user_entity::UserInfo;
use common::errors::AppError;
use common::repository_util::Repository;
use crate::handlers::user_handler::user_list;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(path_list);
}

#[get("/status")]
pub async fn path_list(path_service:web::Data<PathService>) -> Result<impl Responder, AppError> {
    Ok(web::Json(result()))
}
