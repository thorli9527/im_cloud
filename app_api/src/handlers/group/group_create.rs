use actix_web::{web, Responder};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;
use biz_service::biz_service::agent_service::{AgentService, AuthHeader};
use biz_service::biz_service::group_member_service::GroupMemberService;
use biz_service::biz_service::group_service::GroupService;
use biz_service::entitys::group_entity::GroupInfo;
use biz_service::entitys::group_member::{GroupMember, GroupRole};
use common::errors::AppError;
use common::errors::AppError::BizError;
use common::repository_util::Repository;
use common::util::date_util::now;
use crate::result::result;

/// 创建群组请求体
#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateGroupDto {
    /// 群主用户 ID
    #[schema(example = "user_123")]
    pub user_id: String,

    /// 群组唯一 ID
    #[schema(example = "group_001")]
    pub group_id: String,

    /// 群名称
    #[schema(example = "Rust爱好者交流群")]
    pub group_name: String,

    /// 初始成员 ID 列表（不含群主）
    #[schema(example = json!(["user_456", "user_789"]))]
    #[serde(default)]
    pub members: Vec<String>,
}

///// 创建群组接口（签名验证 + 防重复创建）
pub async fn create_group(dto: web::Json<CreateGroupDto>, auth_header: web::Header<AuthHeader>) -> Result<impl Responder, AppError> {
    // ✅ 1. 签名验证
    let (agent, check_state) = AgentService::get().checksum_request(&*auth_header).await?;
    if !check_state {
        return Err(BizError("signature.error".to_string()));
    }

    // ✅ 2. 服务初始化
    let group_service = GroupService::get();
    let member_service = GroupMemberService::get();
    let now = now();
    // ✅ 3. 创建群组
    let group = GroupInfo {
        id:"".to_string(),
        group_id: dto.group_id.clone(),
        agent_id: agent.id.clone(),
        name: dto.group_name.clone(),
        icon_url: None,
        notice: None,
        creator_id: dto.user_id.to_string(),
        group_type: 0,
        max_members: 500,
        status: 1,
        create_time: now,
        update_time: now,
    };
    group_service.dao.insert(&group).await?;

    // ✅ 4. 添加群主
    let owner = GroupMember {
        id:"".to_string(),
        group_id: dto.group_id.clone(),
        user_id: dto.user_id.clone(),
        role: GroupRole::Owner,
        alias: None,
        mute:false,
        mute_until: None,
        create_time: now,
        update_time: now,
    };
    member_service.dao.insert(&owner).await?;

    // ✅ 5. 添加其他初始成员
    for user_id in &dto.members {
        if user_id == &dto.user_id {
            continue; // 跳过群主
        }
        let member = GroupMember {
            id:"".to_string(),
            group_id: dto.group_id.clone(),
            user_id: user_id.clone(),
            role: GroupRole::Member,
            alias: None,
            mute:false,
            mute_until: None,
            create_time: now,
            update_time: now,
        };
        member_service.dao.insert(&member).await?;
    }

    Ok(web::Json(result()))
}