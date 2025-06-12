use crate::result::{result, AppState};
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
pub fn configure(cfg: &mut web::ServiceConfig, state: &web::Data<AppState>) {

}
#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SetGroupMuteDto {
    pub group_id: String,
    pub mute: bool, // true = 开启全体禁言，false = 取消全体禁言
}

/// 设置群组全体禁言状态
pub async fn set_group_mute(dto: web::Json<SetGroupMuteDto>,   req: HttpRequest) -> Result<impl Responder, AppError> {
    let auth_header = build_header(req);
    let (agent, check_state) = AgentService::get().check_request(auth_header).await?;
    if !check_state {
        return Err(BizError("signature.error".to_string()));
    }
    let group_service = GroupService::get();

    let info = group_service.find_by_group_id(&*dto.group_id).await;
    if info.is_err() {
        return Err(BizError("group.not.found".to_string()));
    }
    GroupService::get().dao.up_property(info.unwrap().id, "mute", &dto.mute).await?;
    GroupOperationLogService::get().add_log(&*dto.group_id, "", None, GroupOperationType::MuteAll).await?;
    Ok(web::Json(result()))
}