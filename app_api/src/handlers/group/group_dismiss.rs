use crate::handlers::common_handler::status;
use crate::result::result;
use actix_web::{web, HttpRequest, Responder};
use biz_service::biz_service::agent_service::{build_header, AgentService};
use biz_service::biz_service::group_member_service::GroupMemberService;
use biz_service::biz_service::group_service::GroupService;
use biz_service::biz_service::mq_group_operation_log_service::GroupOperationLogService;
use biz_service::entitys::mq_group_operation_log::GroupOperationType;
use biz_service::manager::user_manager::RedisUserManager;
use common::errors::AppError;
use common::errors::AppError::BizError;
use common::repository_util::Repository;
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(status);
}
/// 解散群组接口（签名验证 + 防重复解散）
pub async fn group_dismiss(group_id: web::Path<String>,  req: HttpRequest) -> Result<impl Responder, AppError> {
    let auth_header = build_header(req);
    let (agent, check_state) = AgentService::get().check_request(auth_header).await?;
    if !check_state {
        return Err(BizError("signature.error".to_string()));
    }
    let group_service = GroupService::get();

    let info = group_service.find_by_group_id(&*group_id).await;
    if info.is_err(){
        return Err(BizError("group.not.found".to_string()));
    }

    let group_member_service = GroupMemberService::get();
    group_member_service.dao.delete_by_id(&*group_id).await?;
    group_service.dao.delete_by_id(&*group_id).await?;
    RedisUserManager::get().dismiss_group(&*group_id).await?;
    GroupOperationLogService::get().add_log(&*group_id, "", None, GroupOperationType::Dismiss).await?;
    Ok(web::Json(result()))
}