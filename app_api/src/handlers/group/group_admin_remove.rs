use actix_web::{web, Responder};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use biz_service::biz_service::agent_service::AuthHeader;
use biz_service::biz_service::group_member_service::GroupMemberService;
use biz_service::biz_service::mq_group_operation_log_service::GroupOperationLogService;
use biz_service::entitys::mq_group_operation_log::GroupOperationType;
use common::errors::AppError;
use common::errors::AppError::BizError;
use crate::result::result;

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RemoveAdminDto {
    /// 群组 ID
    pub group_id: String,

    /// 用户 ID（要设置为管理员的用户）
    pub user_id: String,
}

pub async fn remove_admin_group(
    dto: web::Json<RemoveAdminDto>,
    auth_header: web::Header<AuthHeader>,
) -> Result<impl Responder, AppError> {
    // 签名校验
    let (_agent, valid) = biz_service::biz_service::agent_service::AgentService::get()
        .checksum_request(&*auth_header)
        .await?;
    if !valid {
        return Err(BizError("signature.error".to_string()));
    }

    // 调用 Service 取消管理员
    GroupMemberService::get()
        .remove_admin(&dto.group_id, &dto.user_id)
        .await?;
    GroupOperationLogService::get().add_log(&*dto.group_id, &*dto.user_id, None, GroupOperationType::Demote).await?;
    Ok(web::Json(result()))
}