use actix_web::{get, web, Responder};
use biz_service::biz_services::user_service::UserService;
use biz_service::entitys::user_entity::UserInfo;
use common::errors::AppError;
use common::repository_util::Repository;
use mongodb::bson::DateTime;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use common::util::common_utils::copy_to;
use crate::result::ResultResponse;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(status);
}
#[utoipa::path(
    get,
    path = "/status",
    responses(
        (status = 200, description = "Hello response", body = ResultResponse<UserInfoDto>)
    )
)]
#[get("/status")]
pub async fn status(user_service:web::Data<UserService>) -> Result<impl Responder, AppError> {
    let mut user_info=UserInfo::default();
    let mut result=UserInfoDto::default();
    copy_to(&user_info,&result);
    Ok(web::Json(ResultResponse::ok(Option::Some(result))))
}
#[derive(Debug, Clone, Serialize, Deserialize,Default,ToSchema)]
pub struct UserInfoDto{
    pub id: String, // ✅ 改为 String
    pub user_name:String,
    pub password:String,
    pub status:bool,
    pub is_admin:bool,
    pub create_time: String,
}
