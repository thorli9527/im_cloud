use crate::result::result;
use actix_web::{get, web, Responder};
use common::errors::AppError;
use utoipa::ToSchema;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(status);
}
#[utoipa::path(
    get,
    path = "/status",
    responses(
        (status = 200, description = "Hello response", body = String)
    )
)]
#[get("/status")]
pub async fn status() -> Result<impl Responder, AppError> {
    Ok(result())
}
