use crate::result::{result, AppState};
use actix_web::{get, web, Responder};
use mongodb::bson::DateTime;
use biz_service::biz_services::bucket_service::BucketService;
use biz_service::biz_services::path_service::PathService;
use biz_service::biz_services::user_service;
use biz_service::biz_services::user_service::UserService;
use biz_service::entitys::user_entity::UserInfo;
use common::errors::AppError;
use common::repository_util::Repository;
use crate::handlers::path_handler::path_list;
use crate::handlers::user_handler::user_list;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(bucket_list);
}

#[get("/status")]
pub async fn bucket_list(bucket_service:web::Data<BucketService>) -> Result<impl Responder, AppError> {
    Ok(web::Json(result()))
}
