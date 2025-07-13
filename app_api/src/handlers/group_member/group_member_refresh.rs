use crate::result::{result, ApiResponse};
use actix_web::{post, web, HttpRequest, Responder};
use biz_service::biz_service::agent_service::{build_header, AgentService};
use biz_service::biz_service::group_member_service::GroupMemberService;
use biz_service::entitys::group_member::{GroupMember, GroupRole};
use biz_service::manager::group_manager_core::{GroupManager, GroupManagerOpt};
use common::errors::AppError;
use common::repository_util::Repository;
use common::util::date_util::now;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(group_member_refresh);
}
/// 加入群组请求体
#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct GroupMemberRefreshDto {
    /// 群组 ID
    #[schema(example = "group_001")]
    pub group_id: String,
    /// 用户 ID
    #[schema(example = "user_123")]
    pub user_id: String,
    /// 群内昵称（可选）
    #[schema(example = "铁汁")]
    pub alias: Option<String>,
    /// 角色（可选，默认为 Member）
    pub role: Option<GroupRole>,
}
impl Default for GroupMemberRefreshDto {
    fn default() -> Self {
        GroupMemberRefreshDto { user_id: "".to_string(), group_id: "".to_string(), alias: None, role: None }
    }
}

/// 加入群组接口（签名验证 + 防重复加入）
///
/// 默认角色为 `Member`

#[utoipa::path(
    post,
    path = "/group/member/refresh",
    request_body = GroupMemberRefreshDto,
    summary = "刷新群组成员",
    tag = "群成员管理",
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
#[post("/group/member/refresh")]
async fn group_member_refresh(dto: web::Json<GroupMemberRefreshDto>, req: HttpRequest) -> Result<impl Responder, AppError> {
    let auth_header = build_header(req);
    let agent = AgentService::get().check_request(auth_header).await?;

    let group_manager = GroupManager::get();
    let now = now();

    let exists = group_manager.is_user_in_group(&dto.group_id, &dto.user_id).await?;
    if exists {
        return Ok(web::Json(result()));
    }

    //添加用户到组
    group_manager.group_member_refresh(&dto.group_id, &dto.user_id, Option::None, &dto.alias, &dto.role).await?;

    // ✅ 插入成员记录
    let member = GroupMember {
        id: "".to_string(),
        group_id: dto.group_id.clone(),
        uid: dto.user_id.clone(),
        role: GroupRole::Member,
        alias: dto.alias.clone(),
        mute: false,
        create_time: now,
        update_time: now,
    };
    //更新用户的群组列表
    GroupMemberService::get().dao.insert(&member).await?;

    Ok(web::Json(result()))
}
