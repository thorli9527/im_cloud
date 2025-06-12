use crate::handlers::common_handler::status;
use crate::result::result;
use actix_web::{web, HttpRequest, Responder};
use biz_service::biz_service::agent_service::{build_header, AgentService};
use biz_service::biz_service::group_member_service::GroupMemberService;
use biz_service::biz_service::mq_group_operation_log_service::GroupOperationLogService;
use biz_service::entitys::mq_group_operation_log::GroupOperationType;
use common::errors::AppError;
use common::errors::AppError::BizError;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(status);
}
#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AddAdminDto {
    /// 群组 ID
    pub group_id: String,

    /// 用户 ID（要设置为管理员的用户）
    pub user_id: String,
}



pub async fn group_admin_add(
    dto: web::Json<AddAdminDto>,
    req: HttpRequest) -> Result<impl Responder, AppError> {
    let auth_header = build_header(req);
    let (agent, check_state) = AgentService::get().check_request(auth_header).await?;
    if !check_state {
        return Err(BizError("signature.error".to_string()));
    }

    // 2. 设置管理员
    GroupMemberService::get()
        .add_admin(&dto.group_id, &dto.user_id)
        .await?;
    GroupOperationLogService::get().add_log(&*dto.group_id, &*dto.user_id, None, GroupOperationType::Promote).await?;
    Ok(web::Json(result()))
}