use crate::result::{result, AppState};
use actix_web::{web, HttpRequest, Responder};
use biz_service::biz_service::agent_service::{build_header, AgentService};
use biz_service::biz_service::group_member_service::GroupMemberService;
use biz_service::biz_service::mq_group_operation_log_service::GroupOperationLogService;
use biz_service::entitys::mq_group_operation_log::GroupOperationType;
use common::errors::AppError;
use common::errors::AppError::BizError;
use common::repository_util::Repository;
use common::util::date_util::now;
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
pub fn configure(cfg: &mut web::ServiceConfig, state: &web::Data<AppState>) {
    
}
/// 添加或移除群组禁言白名单成员的请求体
#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WhiteListUserDto {
    /// 群组 ID
    pub group_id: String,

    /// 用户 ID（要添加/移除白名单的用户）
    pub user_id: String,
}
/// 取消指定群成员禁言
pub async fn cancel_mute_member(
    dto: web::Json<WhiteListUserDto>,
    req: HttpRequest) -> Result<impl Responder, AppError> {
    let auth_header = build_header(req);
    let (agent, check_state) = AgentService::get().check_request(auth_header).await?;
    if !check_state {
        return Err(BizError("signature.error".to_string()));
    }
    GroupMemberService::get()
        .dao
        .update(doc! {"group_id":&*dto.group_id,"user_id":&*dto.user_id},
                doc! {
                "mute": false,
                "mute_end_time": Option::<i64>::None,
                "update_time": now(),
            }
        ).await?;

    GroupOperationLogService::get().add_log(&*dto.group_id, &*dto.user_id, None, GroupOperationType::Unmute).await?;
    Ok(web::Json(result()))
}