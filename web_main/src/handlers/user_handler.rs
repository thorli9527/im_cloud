use actix_web::{get, post, web, Responder};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use validator::Validate;
use biz_service::biz_services::user_service::UserService;
use mongodb::bson;
use mongodb::bson::doc;
use common::errors::AppError;
use common::repository_util::{OrderType, PageResult, Repository};
use r#macro::QueryFilter;
use crate::handlers::LoginInfoDto;
use crate::result::{result_data, ApiResponse};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(user_list);
}
#[derive(QueryFilter, Serialize, Deserialize, Debug, Validate, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserInfoQueryDto {
    #[query(eq)]
    pub user_name: Option<String>,
    #[query(gt,field="_id")]
    pub max_id: Option<String>,
    pub page_size: i64,
}

pub struct UserInfoResultDto{

}

#[post("/user/list")]
pub async fn user_list(dto: web::Json<UserInfoQueryDto>, user_service: web::Data<UserService>) -> Result<impl Responder, AppError> {
    let page_result = user_service.dao.query_by_page(doc!{}, dto.page_size, Option::Some(OrderType::Asc),"id").await?;
    Ok(web::Json(result_data(page_result)))
}
