use actix_web::{get, web, Responder};
use biz_service::biz_services::user_service::UserService;
use biz_service::entitys::user_entity::UserInfo;
use common::errors::AppError;
use common::repository_util::Repository;
use mongodb::bson::DateTime;
use crate::result::ResultResponse;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(status);
}

#[get("/status")]
pub async fn status(user_service:web::Data<UserService>) -> Result<impl Responder, AppError> {
    let mut user_info=UserInfo::default();
    Ok(web::Json(ResultResponse::ok(Option::<String>::None)))
}
