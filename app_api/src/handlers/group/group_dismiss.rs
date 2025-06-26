use crate::handlers::common_handler::status;
use crate::result::{ApiResponse, result};
use actix_web::{HttpRequest, Responder, post, web};
use biz_service::biz_service::agent_service::{AgentService, build_header};
use biz_service::biz_service::group_member_service::GroupMemberService;
use biz_service::biz_service::group_service::GroupService;
use biz_service::biz_service::mq_group_operation_log_service::GroupOperationLogService;
use biz_service::entitys::mq_group_operation_log::GroupOperationType;
use biz_service::manager::group_manager_core::{GroupManager, GroupManagerOpt};
use common::errors::AppError;
use common::errors::AppError::BizError;
use common::repository_util::Repository;
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(status);
}
#[utoipa::path(
    post,
    path = "/group/dismiss/{group_id}",
    summary = "解散群组",
    tag = "群管理",
    params(
        ("appKey" = String, Header, description = "应用 key"),
        ("nonce" = String, Header, description = "随机字符串"),
        ("timestamp" = i64, Header, description = "时间戳"),
        ("signature" = String, Header, description = "签名"),
        ("group_id" = String, Path, description = "群组 ID")
    ),
    responses(
        (status = 200, description = "群组解散成功", body = ApiResponse<String>)
    )
)]
#[post("/group/dismiss/{group_id}")]
async fn group_dismiss(group_id: web::Path<String>, req: HttpRequest) -> Result<impl Responder, AppError> {
    let auth_header = build_header(req);
    let agent = AgentService::get().check_request(auth_header).await?;
    let group_service = GroupService::get();

    let info = group_service.find_by_group_id(&*group_id).await;
    if info.is_err() {
        return Err(BizError("group.not.found".to_string()));
    }

    let group_member_service = GroupMemberService::get();
    group_member_service.dao.delete_by_id(&*group_id).await?;
    group_service.dao.delete_by_id(&*group_id).await?;
    GroupManager::get().dismiss_group(&*group_id).await?;
    GroupOperationLogService::get().add_log(&agent.id, &*group_id, "", None, GroupOperationType::Dismiss).await?;
    Ok(web::Json(result()))
}
