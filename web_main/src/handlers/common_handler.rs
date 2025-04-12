use crate::result::result;
use actix_web::{get, web, Responder};
use biz_service::biz_services::user_service::UserService;
use biz_service::entitys::user_entity::UserInfo;
use common::errors::AppError;
use common::repository_util::Repository;
use mongodb::bson::DateTime;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(status);
}

#[get("/status")]
pub async fn status(user_service:web::Data<UserService>) -> Result<impl Responder, AppError> {
    let mut user_info=UserInfo::default();
    user_info.user_name=Some("fdsa".to_string());
    user_info.status=Some(true);
    user_info.is_admin=Some(true);
    user_info.create_time=Some(DateTime::now());
    user_service.dao.insert(&user_info).await;
    let x = user_service.dao.query_all().await?;
    Ok(web::Json(result()))
}
