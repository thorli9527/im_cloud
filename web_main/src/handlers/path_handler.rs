use crate::result::{result, ApiResponse};
use actix_web::{get, web, Responder};
use biz_service::biz_services::path_service::PathService;
use common::errors::AppError;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(path_list);
}

#[get("/status")]
pub async fn path_list(path_service:web::Data<PathService>) -> Result<impl Responder, AppError> {
    return Ok(web::Json(result()));
}
