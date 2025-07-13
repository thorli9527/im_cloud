use crate::handlers::common_handler::status;
use crate::result::{result, ApiResponse};
use actix_web::{post, web, HttpRequest, Responder};
use biz_service::biz_service::agent_service::{build_header, AgentService};
use biz_service::biz_service::group_member_service::GroupMemberService;
use biz_service::biz_service::group_service::GroupService;
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
    let group_id = group_id.into_inner();

    // 1. 构建并校验身份
    let auth_header = build_header(req);
    let agent = AgentService::get().check_request(auth_header).await?;

    // 2. 校验群组是否存在
    let group_service = GroupService::get();
    let group_info = match group_service.find_by_group_id(&group_id).await {
        Ok(info) => info,
        Err(_) => return Err(BizError("group.not.found".to_string())),
    };
    group_service.dismiss_group(&group_id, &"");

    // 6. 返回结果
    Ok(web::Json(result()))
}
