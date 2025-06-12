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
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(status);
}
/// 加入群组请求体
#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GroupQuitDto {
    /// 用户 ID
    #[schema(example = "user_123")]
    pub user_id: String,

    /// 群组 ID
    #[schema(example = "group_001")]
    pub group_id: String,
}
/// 退出群组接口（签名验证 + 防重复退出）
pub async fn group_quit(dto: web::Json<GroupQuitDto>,  req: HttpRequest) -> Result<impl Responder, AppError> {
    let auth_header = build_header(req);
    let (agent, check_state) = AgentService::get().check_request(auth_header).await?;
    if !check_state {
        return Err(BizError("signature.error".to_string()));
    }
    let group_service = GroupService::get();

    let info = group_service.find_by_group_id(&*dto.group_id).await;
    if info.is_err(){
        return Err(BizError("group.not.found".to_string()));
    }
    if let Ok(group) = info {
        if group.creator_id != dto.user_id {
            return Err(BizError("user.group.owner".to_string()));
        }
    }

    let member_service = GroupMemberService::get();

    let redis_user_manager = RedisUserManager::get();
    redis_user_manager.remove_from_group(&dto.group_id, &dto.user_id).await?;


    member_service.remove(&dto.group_id, &dto.user_id).await?;
    //发送消息
    GroupOperationLogService::get().add_log(&dto.group_id, &dto.user_id, None, GroupOperationType::Quit).await?;
    Ok(web::Json(result()))
}