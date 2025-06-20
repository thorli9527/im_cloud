use crate::result::{result, result_data, AppState};
use actix_web::{get, web, HttpRequest, Responder};
use biz_service::biz_service::agent_service::{build_header, AgentService};
use biz_service::biz_service::group_member_service::GroupMemberService;
use biz_service::biz_service::group_service::GroupService;
use biz_service::biz_service::mq_group_operation_log_service::GroupOperationLogService;
use biz_service::entitys::mq_group_operation_log::{GroupOperationLog, GroupOperationType};
use common::errors::AppError;
use common::repository_util::Repository;
use common::util::common_utils::as_ref_to_string;
use common::util::date_util::now;
use mongodb::bson::doc;
pub fn configure(cfg: &mut web::ServiceConfig, state: &web::Data<AppState>) {

}
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
    req: HttpRequest) -> Result<impl Responder, AppError> {
    let auth_header = build_header(req);
    let _= AgentService::get().check_request(auth_header).await?;
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