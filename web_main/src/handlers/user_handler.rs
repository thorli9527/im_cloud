use crate::handlers::LoginInfoDto;
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

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(user_list);
    cfg.service(user_add);
}
#[derive(QueryFilter, Serialize, Deserialize, Debug, Validate, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserInfoQueryDto {
    #[query(eq)]
    pub user_name: Option<String>,
    #[query(gt, field = "_id")]
    pub max_id: Option<String>,
    pub page_size: i64,
}

pub struct UserInfoResultDto {}

#[post("/user/list")]
pub async fn user_list(dto: web::Json<UserInfoQueryDto>, user_service: web::Data<UserService>) -> Result<impl Responder, AppError> {
    let page_result = user_service.dao.query_by_page(dto.to_query_doc(), dto.page_size, Option::Some(OrderType::Asc), "_id").await?;
    Ok(web::Json(result_data(page_result)))
}

#[derive(Serialize, Deserialize, Debug, Validate, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserAddDto {
    #[validate(length(min = 5, message = "用户名太短"))]
    pub user_name: Option<String>,
    #[validate(length(min = 5, message = "密码太短"))]
    pub password: Option<String>,
    pub is_admin: bool,
}
#[post("/user/add")]
pub async fn user_add(state: web::Data<AppState>, dto: web::Json<UserAddDto>, user_service: web::Data<UserService>) -> Result<impl Responder, AppError> {
    match &dto.validate() {
        Ok(_) => {
            let mut user = UserInfo::default();
            user.user_name = dto.user_name.as_ref().unwrap().to_string();
            user.password = build_md5_with_key(&state.config.sys.md5_key, &dto.password.as_ref().unwrap());
            user.is_admin = dto.is_admin.clone();
            user.status = true;
            user.create_time = build_now();
            user_service.dao.insert(&user).await?;
            Ok(web::Json(result()))
        }
        Err(e) => return Ok(web::Json(result_error_msg(e.to_string().as_str()))),
    }

}
#[post("/user/change/{user_id}/{state}")]
pub async fn user_change(parmas: web::Path<(String, bool)>, user_service: web::Data<UserService>) -> Result<impl Responder, AppError> {
    let (user_id, state) = parmas.into_inner();
    user_service.dao.up_property(user_id, "status".to_string(), state);
    Ok(web::Json(result()))
}

#[post("/user/del/{user_id}")]
pub async fn user_del(user_id: web::Path<String>, user_service: web::Data<UserService>) -> Result<impl Responder, AppError> {
    user_service.dao.delete_by_id(user_id.to_string());
    Ok(web::Json(result()))
}
