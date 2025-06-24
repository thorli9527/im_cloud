use crate::result::{result_data, ApiResponse,};
use actix_web::{post, web, HttpRequest, Responder};
use biz_service::biz_service::agent_service::{build_header, AgentService};
use biz_service::manager::group_redis_manager::{GroupManager, GroupManagerOpt};
use common::errors::AppError;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub fn configure(cfg: &mut web::ServiceConfig) {

}
/// 拉取群组成员分页查询请求体
#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GroupMemberPageDto {
    /// 群组 ID
    pub group_id: String,

    /// 页码（从 1 开始）
    pub page: u64,

    /// 每页大小
    pub size: u64,
}

#[utoipa::path(
    post,
    path = "/group/member/page",
    request_body = GroupMemberPageDto,
    tag = "群成员管理",
    summary = "分页查询群组成员",
    params(
        ("appKey" = String, Header, description = "应用 key"),
        ("nonce" = String, Header, description = "随机字符串"),
        ("timestamp" = i64, Header, description = "时间戳"),
        ("signature" = String, Header, description = "签名")
    ),
    responses(
        (status = 200, description = "分页查询群组成员成功", body = ApiResponse<Vec<String>>)
    )
)]
#[post("/group/member/page")]
pub async fn group_member_page(
    query: web::Json<GroupMemberPageDto>,
    req: HttpRequest) -> Result<impl Responder, AppError> {
    let auth_header = build_header(req);
    let _ = AgentService::get().check_request(auth_header).await?;

    // 2. 分页查询
    let members = GroupManager::get().get_group_members_by_page(&query.group_id,query.page as usize,query.size as usize).await?;

    // 3. 返回
    Ok(web::Json(result_data(members)))
}
