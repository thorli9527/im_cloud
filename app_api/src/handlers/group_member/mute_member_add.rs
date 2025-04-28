use actix_web::{web, Responder};
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use biz_service::biz_service::agent_service::{AgentService, AuthHeader};
use biz_service::biz_service::group_member_service::GroupMemberService;
use biz_service::biz_service::mq_group_operation_log_service::GroupOperationLogService;
use biz_service::entitys::mq_group_operation_log::GroupOperationType;
use common::errors::AppError;
use common::errors::AppError::BizError;
use common::repository_util::Repository;
use common::util::date_util::now;
use crate::result::result;

/// 取消禁言成员请求体
#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MuteMemberDto {
    /// 群组 ID
    pub group_id: String,

    /// 用户 ID
    pub user_id: String,

    /// 是否禁言（true=禁言，false=取消禁言）
    pub mute: bool,

}
pub async fn mute_member_add(dto: web::Json<MuteMemberDto>, auth_header: web::Header<AuthHeader>) -> Result<impl Responder, AppError> {
    let (_agent, valid) = AgentService::get().checksum_request(&*auth_header).await?;
    if !valid {
        return Err(BizError("signature.error".to_string()));
    }
    let mut update_doc=doc! {};
    if dto.mute {
        update_doc.insert("mute", true);
    }

    GroupMemberService::get().dao.update(doc! {"group_id":&*dto.group_id,"user_id":&*dto.user_id}, update_doc).await?;
    GroupOperationLogService::get().add_log(&*dto.group_id, &*dto.user_id, None, GroupOperationType::Mute).await?;
    Ok(web::Json(result()))
}