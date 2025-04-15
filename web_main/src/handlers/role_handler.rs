use crate::result::{ApiResponse, AppState, result, result_data, result_error_msg};
use actix_web::web::Json;
use actix_web::{Responder, get, post, web};
use biz_service::biz_services::user_service::UserService;
use biz_service::entitys::user_entity::UserInfo;
use common::errors::AppError;
use common::repository_util::{OrderType, PageResult, Repository};
use common::util::common_utils::build_md5_with_key;
use common::util::date_util::build_now;
use r#macro::QueryFilter;
use mongodb::bson;
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use validator::Validate;
use biz_service::biz_services::role_service::RoleService;
use crate::handlers::{user_add, UserInfoQueryDto};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(role_list);
}


pub struct UserInfoResultDto {}

#[post("/role/list")]
pub async fn role_list(role_service: web::Data<RoleService>) -> Result<impl Responder, AppError> {
    Ok(web::Json(result()))
}
#[post("/role/add")]
pub async fn role_add(role_service: web::Data<RoleService>) -> Result<impl Responder, AppError> {
    Ok(web::Json(result()))
}
#[post("/role/del")]
pub async fn role_del(role_service: web::Data<RoleService>) -> Result<impl Responder, AppError> {
    Ok(web::Json(result()))
}

