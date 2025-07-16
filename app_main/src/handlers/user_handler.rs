use crate::result::{result, result_data, result_error, AppState};
use actix_web::{post, web, Responder};
use biz_service::biz_service::user_service::UserService;
use biz_service::entitys::user_entity::UserInfoEntity;
use common::errors::AppError;
use common::repository_util::{OrderType, Repository};
use common::util::common_utils::build_md5_with_key;
use common::util::date_util::now;
use mongodb::bson;
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;
use web::Json;
use common::config::AppConfig;
use mongodb::bson::Document;
use mongo_macro::QueryFilter;

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

#[post("/user/list")]
pub async fn user_list(dto: Json<UserInfoQueryDto>) -> Result<impl Responder, AppError> {
    let page_result = UserService::get().dao.query_by_page(dto.to_query_doc(), dto.page_size, Option::Some(OrderType::Asc), "_id").await?;
    Ok(Json(result_data(page_result)))
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
pub async fn user_add(dto: Json<UserAddDto>) -> Result<impl Responder, AppError> {
    match &dto.validate() {
        Ok(_) => {
            let mut user = UserInfoEntity::default();
            let sys_config = AppConfig::get().clone().sys.clone().unwrap();
            let md5_key = &sys_config.md5_key.unwrap();
            user.user_name = dto.user_name.as_ref().unwrap().to_string();
            user.password = build_md5_with_key(&md5_key, &dto.password.as_ref().unwrap());
            user.is_admin = dto.is_admin.clone();
            user.status = true;
            user.create_time = now();
            UserService::get().dao.insert(&user).await?;
            Ok(result())
        }
        Err(e) => return Ok(result_error(e.to_string())),
    }
}
#[post("/user/change/{user_id}/{state}")]
pub async fn user_change(parmas: web::Path<(String, bool)>) -> Result<impl Responder, AppError> {
    let (user_id, state) = parmas.into_inner();
    UserService::get().dao.up_property(&user_id, "status", state).await?;
    Ok(result())
}

#[post("/user/del/{user_id}")]
pub async fn user_del(user_id: web::Path<String>) -> Result<impl Responder, AppError> {
    UserService::get().dao.delete_by_id(&user_id).await?;
    Ok(result())
}

#[derive(Serialize, Deserialize, Debug, Validate, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserPassChange {
    pub user_id: String,
    #[validate(length(min = 5, message = "密码太短"))]
    pub password: Option<String>,
}

#[post("/user/change/pass")]
pub async fn user_change_pass(state: web::Data<AppState>, dto: Json<UserPassChange>) -> Result<impl Responder, AppError> {
    match &dto.validate() {
        Ok(_) => {
            let password = build_md5_with_key(&state.config.get_sys().md5_key.unwrap(), &dto.password.as_ref().unwrap());
            UserService::get().dao.up_property(&dto.user_id, "status", password).await?;
            Ok(result())
        }
        Err(e) => return Ok(result_error(e.to_string())),
    }
}
