use crate::handlers::common_handler::status;
use crate::result::result;
use actix_web::{web, HttpRequest, Responder};
use biz_service::biz_service::agent_service::{build_header, AgentService};
use biz_service::biz_service::group_service::GroupService;
use biz_service::biz_service::mq_group_operation_log_service::GroupOperationLogService;
use biz_service::entitys::mq_group_operation_log::GroupOperationType;
use common::errors::AppError;
use common::errors::AppError::BizError;
use common::repository_util::Repository;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(status);
}
#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct GroupRefreshDto {
    /// 群组 ID
    #[schema(example = "group_001")]
    pub group_id: String,

    /// 群组名称
    #[schema(example = "Rust爱好者交流群")]
    pub group_name: String,
}
/// 修改群组名称请求体
pub async fn group_refresh(dto: web::Json<GroupRefreshDto>,  req: HttpRequest) -> Result<impl Responder, AppError> {
    let auth_header = build_header(req);
    let (agent, check_state) = AgentService::get().check_request(auth_header).await?;
    if !check_state {
        return Err(BizError("signature.error".to_string()));
    }
    let group_service = GroupService::get();
    let info = group_service.find_by_group_id(&*dto.group_id).await;
    if info.is_err(){
        return Err(BizError("group.not.found".to_string()));
    }
    group_service.dao.up_property(info.ok().unwrap().id, "name", &dto.group_name).await?;

    GroupOperationLogService::get().add_log(&*dto.group_id, "", None, GroupOperationType::Change).await?;
    Ok(web::Json(result()))
}