use actix_web::{get, web, Responder};
use mongodb::bson::doc;
use biz_service::biz_service::agent_service::{AgentService, AuthHeader};
use biz_service::biz_service::group_member_service::GroupMemberService;
use biz_service::biz_service::group_service::GroupService;
use biz_service::biz_service::mq_group_operation_log_service::GroupOperationLogService;
use biz_service::entitys::mq_group_operation_log::{GroupOperationLog, GroupOperationType};
use common::errors::AppError;
use common::errors::AppError::BizError;
use common::repository_util::Repository;
use common::util::common_utils::as_ref_to_string;
use common::util::date_util::now;
use crate::result::{result, result_data};

/// 查询群组内被禁言的成员列表
#[get("/group/mute/member/list/{group_id}")]
pub async fn list_muted_members(
    group_id: web::Path<String>
) -> Result<impl Responder, AppError> {
    let now = now();
    let members = GroupMemberService::get()
        .dao
        .query(doc! {
            "group_id": group_id.as_str(),
            "mute": true,
        }).await?;
    Ok(web::Json(result_data(members)))
}

/// 取消群组全体禁言
pub async fn cancel_group_mute(
    group_id: web::Path<String>,
    auth_header: web::Header<AuthHeader>
) -> Result<impl Responder, AppError> {
    let (_agent, valid) = AgentService::get().checksum_request(&*auth_header).await?;
    if !valid {
        return Err(BizError("signature.error".to_string()));
    }
    GroupService::get()
        .dao
        .up_property(group_id.as_str(), "mute", &false)
        .await?;

    let mut log = GroupOperationLog::default();
    log.group_id =as_ref_to_string(&*group_id);
    log.operator_id = "system".to_string();
    log.sync_statue = false;
    log.action= GroupOperationType::UnmuteAll;
    GroupOperationLogService::get().dao.insert(&log).await?;
    Ok(web::Json(result()))
}