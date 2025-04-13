use actix_web::{get, web, Responder};
use biz_service::biz_services::user_service::UserService;

use common::errors::AppError;
use crate::result::ResultResponse;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(user_list);
}

#[get("/status")]
pub async fn user_list(user_service:web::Data<UserService>) -> Result<impl Responder, AppError> {

    Ok(web::Json(ResultResponse::<String>::ok(Option::None)))
}
