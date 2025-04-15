use actix_web::{get, web, Responder};
use biz_service::biz_services::user_service::UserService;
use biz_service::entitys::user_entity::UserInfo;
use common::errors::AppError;
use common::repository_util::Repository;
use mongodb::bson::DateTime;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use common::util::common_utils::copy_to;
use crate::result::{result, ApiResponse};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(status);
}
#[utoipa::path(
    get,
    path = "/status",
    responses(
        (status = 200, description = "Hello response", body = ApiResponse<String>)
    )
)]
#[get("/status")]
pub async fn status(user_service:web::Data<UserService>) -> Result<impl Responder, AppError> {
    return Ok(web::Json(result()));
}
