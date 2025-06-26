use crate::result::{ApiResponse, result};
use actix_web::{HttpRequest, Responder, post, web};
use biz_service::biz_service::agent_service::{AgentService, build_header};
use biz_service::biz_service::group_member_service::GroupMemberService;
use biz_service::biz_service::group_service::GroupService;
use biz_service::entitys::group_entity::GroupInfo;
use biz_service::entitys::group_member::{GroupMember, GroupRole};
use common::errors::AppError;
use common::repository_util::Repository;
use common::util::common_utils::build_uuid;
use common::util::date_util::now;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(group_create);
}
/// 创建群组请求体
#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct CreateGroupDto {
    /// 群主用户 ID
    #[schema(example = "user_123")]
    pub user_id: String,

    /// 群名称
    #[schema(example = "Rust爱好者交流群")]
    pub group_name: String,

    /// 初始成员 ID 列表（不含群主）
    #[serde(default)]
    pub members: Vec<String>,
}
#[utoipa::path(
    post,
    path = "/group/create",
    summary = "创建群组",
    tag = "群管理",
    params(
        ("appKey" = String, Header, description = "应用 key"),
        ("nonce" = String, Header, description = "随机字符串"),
        ("timestamp" = i64, Header, description = "时间戳"),
        ("signature" = String, Header, description = "签名")
    ),
    request_body = CreateGroupDto,
    responses(
        (status = 200, description = "Hello response", body = ApiResponse<String>)
    )
)]
#[post("/group/create")]
async fn group_create(dto: web::Json<CreateGroupDto>, req: HttpRequest) -> Result<impl Responder, AppError> {
    let auth_header = build_header(req);
    let agent = AgentService::get().check_request(auth_header).await?;

    // ✅ 2. 服务初始化
    let group_service = GroupService::get();
    let member_service = GroupMemberService::get();
    let now = now();
    let group_id = build_uuid();
    // ✅ 3. 创建群组
    let group = GroupInfo {
        id: group_id.clone(),
        group_id: group_id.clone(),
        agent_id: agent.id.clone(),
        name: dto.group_name.clone(),
        avatar: None,
        description: None,
        announcement: None,
        owner_id: dto.user_id.to_string(),
        group_type: 0,
        max_members: 500,
        status: 1,
        create_time: now,
        update_time: now,
    };
    group_service.dao.insert(&group).await?;

    // ✅ 4. 添加群主
    let owner = GroupMember { id: "".to_string(), group_id: group_id.clone(), uid: dto.user_id.clone(), role: GroupRole::Owner, alias: None, mute: false, create_time: now, update_time: now };
    member_service.dao.insert(&owner).await?;

    // ✅ 5. 添加其他初始成员
    for user_id in &dto.members {
        if user_id == &dto.user_id {
            continue; // 跳过群主
        }
        let member = GroupMember { id: "".to_string(), group_id: group_id.clone(), uid: user_id.clone(), role: GroupRole::Member, alias: None, mute: false, create_time: now, update_time: now };
        member_service.dao.insert(&member).await?;
    }

    Ok(web::Json(result()))
}
