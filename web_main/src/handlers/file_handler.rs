use crate::result::{result, ApiResponse};
use actix_web::{get, web, Responder};
use biz_service::biz_services::file_service::FileService;
use common::errors::AppError;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(bucket_list);
}

#[get("/status")]
pub async fn bucket_list(file_service:web::Data<FileService>) -> Result<impl Responder, AppError> {
    return Ok(web::Json(result()));
}
