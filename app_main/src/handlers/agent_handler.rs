use crate::result::{result, result_page, ApiResponse};
use actix_web::{post, web, Responder};
use anyhow::{anyhow, Result};
use biz_service::biz_service::agent_service::AgentService;
use biz_service::entitys::agent_entity::AgentInfo;
use chrono::Utc;
use common::errors::AppError;
use common::query_builder::PageInfo;
use common::repository_util::{PageResult, Repository};
use common::util::common_utils::build_uuid;
use common::util::date_util::now;
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(agent_list);
    cfg.service(agent_refresh_secret);
    cfg.service(agent_create);
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
    let page_result = agent_service
        .dao
        .query_by_page(doc! {}, page.page_size as i64, Option::Some(page.order_type.clone()), "create_time")
        .await?;
    let json_page = serde_json::json!(page_result);
    Ok(result_page(json_page))
}
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct AgentInfoDto {
    /// 代理名称，必填，长度 1~32
    #[validate(length(min = 2, max = 32))]
    pub name: String,

    /// 代理描述，最多 100 字符
    #[validate(length(max = 100))]
    pub remark: Option<String>,
}

#[utoipa::path(
    post,
    path = "/agent/create",
    request_body = AgentInfoDto,
    responses(
        (status = 200, description = "Hello response", body = ApiResponse<String>)
    )
)]
#[post("/agent/create")]
pub async fn agent_create(dto: web::Json<AgentInfoDto>) -> Result<impl Responder, AppError> {
    // 执行验证
    dto.validate()?;
    let agent_service = AgentService::get();
    let mut agent = AgentInfo::default();
    agent.create_time = now();
    match &dto.remark {
        Some(desc) => agent.remark = desc.to_string(),
        None => agent.remark = "".to_string(),
    }
    agent.name = dto.name.clone();
    agent.app_secret = build_uuid();
    agent.app_key = build_uuid();
    agent.enable = true;
    let end_time = now() + 15 * 24 * 60 * 60;
    agent.end_time = end_time; // 默认15天有效期
    agent_service.dao.insert(&agent).await?;
    Ok(result())
}
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct AgentKeyDto {
    #[validate(length(min = 16, message = "agent_id 不能为空，且长度至少为 16"))]
    pub agent_id: String,
}
#[utoipa::path(
    post,
    path = "/agent/refresh_secret",
    request_body = AgentKeyDto,
    responses(
        (status = 200, description = "Hello response", body = ApiResponse<String>)
    )
)]
#[post("/agent/refresh_secret")]
pub async fn agent_refresh_secret(dto: web::Json<AgentKeyDto>) -> Result<impl Responder, AppError> {
    dto.validate()?;
    let agent_service = AgentService::get();
    let entity = agent_service.dao.find_by_id(&dto.agent_id).await?;
    if entity.is_none() {
        return Err(AppError::BizError("agent.not.found".to_string()));
    }
    agent_service.dao.up_property(&dto.agent_id, "app_secret", build_uuid()).await?;
    Ok(result())
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct AgentEndTimeDto {
    #[validate(length(min = 16, message = "agent_id 不能为空，且长度至少为 16"))]
    pub agent_id: String,

    pub end_time: u64,
}
#[utoipa::path(
    post,
    path = "/agent/refresh_end_time",
    request_body = AgentEndTimeDto,
    responses(
        (status = 200, description = "Hello response", body = ApiResponse<String>)
    )
)]
#[post("/agent/refresh_end_time")]
pub async fn agent_refresh_end_time(dto: web::Json<AgentEndTimeDto>) -> Result<impl Responder, AppError> {
    dto.validate()?;
    validate_agent_end_time(&dto)?;
    let agent_service = AgentService::get();
    let entity = agent_service.dao.find_by_id(&dto.agent_id).await?;
    if entity.is_none() {
        return Err(AppError::BizError("agent.not.found".to_string()));
    }
    agent_service.dao.up_property(&dto.agent_id, "end_time", &dto.end_time).await?;
    Ok(result())
}

fn validate_agent_end_time(dto: &AgentEndTimeDto) -> Result<()> {
    let now = Utc::now().timestamp() as u64;
    let min_end_time = now + 86400; // 当前时间 + 1 天
    if dto.end_time <= min_end_time {
        return Err(anyhow!("end_time 必须大于当前时间 + 1 天"));
    }
    Ok(())
}
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct AgentEnable {
    /// 代理 ID，唯一标识
    #[validate(length(min = 16, message = "agent_id 不能为空，且长度至少为 16"))]
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
    dto.validate()?;
    let agent_service = AgentService::get();
    agent_service.dao.up_property(&dto.id, "enable", dto.enable).await?;
    Ok(result())
}
