use crate::result::result;
use actix_web::{get, web, Responder};
use biz_service::biz_services::bucket_service::BucketService;
use common::errors::AppError;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(bucket_list);
}

#[get("/status")]
pub async fn bucket_list(bucket_service:web::Data<BucketService>) -> Result<impl Responder, AppError> {
    Ok(web::Json(result()))
}
