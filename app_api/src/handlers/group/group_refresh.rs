use crate::result::{result, ApiResponse};
use actix_web::{post, web, HttpRequest, Responder};
use biz_service::biz_service::agent_service::{build_header, AgentService};
use biz_service::biz_service::group_service::GroupService;
use common::errors::AppError;
use common::errors::AppError::BizError;
use common::repository_util::Repository;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(group_refresh);
}
#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct GroupRefreshDto {
    /// 群组 ID
    #[schema(example = "group_001")]
    pub group_id: String,

    /// 群组名称
    #[schema(example = "Rust爱好者交流群")]
    pub group_name: String,
    /// 群组头像 URL
    /// #[schema(example = "https://example.com/group_avatar.png")]
    pub group_avatar: Option<String>,
    ///群组简介
    #[schema(example = "这是一个关于 Rust 编程语言的交流群。")]
    pub group_description: Option<String>,
    /// 群组公告
    /// #[schema(example = "欢迎加入 Rust 爱好者交流群！请遵守群规。")]
    pub group_announcement: Option<String>,
}
#[utoipa::path(
    post,
    path = "/group/refresh",
    request_body = GroupRefreshDto,
    tag = "群管理",
    summary = "修改群组名称",
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
#[post("/group/refresh")]
/// 修改群组名称请求体
async fn group_refresh(dto: web::Json<GroupRefreshDto>, req: HttpRequest) -> Result<impl Responder, AppError> {
    let auth_header = build_header(req);
    let agent = AgentService::get().check_request(auth_header).await?;
    let group_service = GroupService::get();
    let info = group_service.find_by_group_id(&*dto.group_id).await;
    if info.is_err() {
        return Err(BizError("group.not.found".to_string()));
    }
    group_service.dao.up_property(&info.ok().unwrap().id, "name", &dto.group_name).await?;

    Ok(web::Json(result()))
}
