use crate::result::{result, ApiResponse};
use actix_web::{post, web, HttpRequest, Responder};
use biz_service::biz_service::agent_service::{build_header, AgentService};
use biz_service::manager::group_manager_core::{GroupManager, GroupManagerOpt};
use common::errors::AppError;
use common::errors::AppError::BizError;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(group_quit);
}
/// 加入群组请求体
#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct GroupQuitDto {
    /// 用户 ID
    #[schema(example = "user_123")]
    pub user_id: String,

    /// 群组 ID
    #[schema(example = "group_001")]
    pub group_id: String,
}
/// 退出群组接口（签名验证 + 防重复退出）
#[utoipa::path(
    post,
    path = "/group/quit",
    tag = "群管理",
    request_body = GroupQuitDto,
    summary = "退出群组",
    params(
        ("appKey" = String, Header, description = "应用 key"),
        ("nonce" = String, Header, description = "随机字符串"),
        ("timestamp" = i64, Header, description = "时间戳"),
        ("signature" = String, Header, description = "签名")
    ),
    responses(
        (status = 200, description = "群组解散成功", body = ApiResponse<String>)
    )
)]
#[post("/group/quit")]
async fn group_quit(dto: web::Json<GroupQuitDto>, req: HttpRequest) -> Result<impl Responder, AppError> {
    let auth_header = build_header(req);
    let _ = AgentService::get().check_request(auth_header).await?;
    let group_manager = GroupManager::get();

    let info = group_manager.get_group_info(&*dto.group_id).await?;
    if info.is_none() {
        return Err(BizError("group.not.found".to_string()));
    }
    // 检查用户是否是群主
    let info = info.unwrap();
    if info.owner_id != dto.user_id {
        return Err(BizError("user.owner".to_string()));
    }
    group_manager.remove_user_from_group(&dto.group_id, &dto.user_id).await?;
    Ok(web::Json(result()))
}
