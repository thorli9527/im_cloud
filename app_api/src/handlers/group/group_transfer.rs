use crate::handlers::common_handler::status;
use crate::result::result;
use actix_web::{web, HttpRequest, Responder};
use biz_service::biz_service::agent_service::{build_header, AgentService};
use biz_service::biz_service::group_service::GroupService;
use biz_service::biz_service::mq_group_operation_log_service::GroupOperationLogService;
use biz_service::entitys::mq_group_operation_log::GroupOperationType;
use common::errors::AppError;
use common::errors::AppError::BizError;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(status);
}
/// 转让群组请求体
#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TransferGroupDto {
    /// 群组 ID
    pub group_id: String,

    /// 新群主用户 ID
    pub new_owner_id: String,
}



pub async fn group_transfer(
    dto: web::Json<TransferGroupDto>,
    req: HttpRequest) -> Result<impl Responder, AppError> {
    let auth_header = build_header(req);
    let (agent, check_state) = AgentService::get().check_request(auth_header).await?;
    if !check_state {
        return Err(BizError("signature.error".to_string()));
    }

    // ✅ 调用 GroupService 更新 creator_id
    GroupService::get()
        .transfer_ownership(&dto.group_id, &dto.new_owner_id)
        .await?;
    GroupOperationLogService::get().add_log(&*dto.group_id, &*dto.new_owner_id, None, GroupOperationType::Transfer).await?;
    Ok(web::Json(result()))
}