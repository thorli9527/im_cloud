use actix_web::{post, web, Responder};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::{path, ToSchema};
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

pub async fn create_group(
    dto: web::Json<CreateGroupDto>,
    auth_header: web::Header<AuthHeader>,
) -> Result<impl Responder, AppError> {
    // ✅ 1. 签名验证
    let (agent,check_state) = AgentService::get().checksum_request(&*auth_header).await?;
    if !check_state {
        return Err(BizError("signature.error".to_string()));
    }

    // ✅ 2. 服务初始化
    let group_service = GroupService::get();
    let member_service = GroupMemberService::get();
    let now = now();

    // ✅ 3. 创建群组
    let group = GroupInfo {
        id: dto.group_id.clone(),
        agent_id:agent.id.clone(),
        name: dto.group_name.clone(),
        icon_url: None,
        notice: None,
        creator_id: dto.user_id.parse().unwrap_or(0),
        group_type: 0,
        member_count: (dto.members.len() + 1) as i32,
        max_members: 500,
        status: 1,
        create_time: now,
        update_time: now,
    };
    group_service.dao.insert(&group).await?;

    // ✅ 4. 添加群主
    let owner = GroupMember {
        id: format!("{}_{}", dto.group_id, dto.user_id),
        group_id: dto.group_id.clone(),
        user_id: dto.user_id.clone(),
        role: GroupRole::Owner,
        alias: None,
        mute_until: 0,
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
            id: format!("{}_{}", dto.group_id, user_id),
            group_id: dto.group_id.clone(),
            user_id: user_id.clone(),
            role: GroupRole::Member,
            alias: None,
            mute_until: 0,
            create_time: now,
            update_time: now,
        };
        member_service.dao.insert(&member).await?;
    }

    Ok(web::Json(result()))
}


/// 加入群组请求体
#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct JoinGroupDto {
    /// 用户 ID
    #[schema(example = "user_123")]
    pub user_id: String,

    /// 群组 ID
    #[schema(example = "group_001")]
    pub group_id: String,

    /// 群内昵称（可选）
    #[schema(example = "铁汁")]
    pub alias: Option<String>,
}

/// 加入群组接口（签名验证 + 防重复加入）
///
/// 默认角色为 `Member`

pub async fn join_group(
    dto: web::Json<JoinGroupDto>,
    auth_header: web::Header<AuthHeader>,
) -> Result<impl Responder, AppError> {
    // ✅ 校验签名
    let (_agent, valid) = AgentService::get()
        .checksum_request(&*auth_header)
        .await?;
    if !valid {
        return Err(BizError("signature.error".to_string()));
    }

    let member_service = GroupMemberService::get();
    let now = now();

    // ✅ 检查是否已加入
    let exists = member_service
        .find_member(&dto.group_id, &dto.user_id)
        .await?
        .is_some();
    if exists {
        return Err(BizError("user.already.in.group".to_string()));
    }

    // ✅ 插入成员记录
    let member = GroupMember {
        id: format!("{}_{}", dto.group_id, dto.user_id),
        group_id: dto.group_id.clone(),
        user_id: dto.user_id.clone(),
        role: GroupRole::Member,
        alias: dto.alias.clone(),
        mute_until: 0,
        create_time: now,
        update_time: now,
    };
    member_service.dao.insert(&member).await?;

    Ok(web::Json(result()))
}