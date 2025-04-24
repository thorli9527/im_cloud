use crate::result::{result, result_page, ApiResponse};
use actix_web::{post, web, Responder};
use biz_service::biz_service::agent_service::AgentService;
use biz_service::entitys::agent_entity::AgentInfo;
use common::errors::AppError;
use common::query_builder::PageInfo;
use common::repository_util::{PageResult, Repository};
use common::util::common_utils::build_uuid;
use common::util::date_util::now;
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(agent_list);
    cfg.service(agent_save);
    cfg.service(agent_active);
}
#[utoipa::path(
    post,
    path = "/agent/list",
  request_body = PageInfo,
    responses(
        (status = 200, description = "Hello response", body = PageResult<AgentInfo>)
    )
)]
#[post("/agent/list")]
pub async fn agent_list(page: web::Json<PageInfo>) -> Result<impl Responder, AppError> {
    let agent_service = AgentService::get();
    let page_result = agent_service.dao.query_by_page(doc! {}, page.page_size as i64, Option::Some(page.order_type.clone()), "create_time").await?;
    let json_page = serde_json::json!(page_result);
    Ok(result_page(json_page))
}

#[utoipa::path(
    post,
    path = "/agent/save",
  request_body = AgentInfo,
    responses(
        (status = 200, description = "Hello response", body = ApiResponse<String>)
    )
)]
#[post("/agent/save")]
pub async fn agent_save(mut dto: web::Json<AgentInfo>) -> Result<impl Responder, AppError> {
    let agent_service = AgentService::get();
    if dto.id.is_empty() {
        dto.create_time = now();
        dto.app_secret = build_uuid();
        dto.app_key = build_uuid();
        dto.enable = true;
        agent_service.dao.insert(&dto).await?;
    } else {
        agent_service.dao.save(&dto).await?;
    }
    Ok(result())
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentEnable {
    /// 代理 ID，唯一标识
    pub id: String,
    /// 是否启用该代理（true 表示启用）
    pub enable: bool,
}

#[utoipa::path(
    post,
    path = "/agent/active",
  request_body = AgentEnable,
    responses(
        (status = 200, description = "Hello response", body = ApiResponse<String>)
    )
)]
#[post("/agent/active")]
pub async fn agent_active(dto: web::Json<AgentEnable>) -> Result<impl Responder, AppError> {
    let agent_service = AgentService::get();
    agent_service.dao.up_property(&dto.id, "enable", dto.enable).await?;
    Ok(result())
}
