use actix_web::{web, Responder};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use biz_service::biz_service::agent_service::{AgentService, AuthHeader};
use biz_service::biz_service::group_member_service::GroupMemberService;
use biz_service::biz_service::mq_group_operation_log_service::GroupOperationLogService;
use biz_service::entitys::group_member::{GroupMember, GroupRole};
use biz_service::entitys::mq_group_operation_log::GroupOperationType;
use biz_service::manager::user_manager::RedisUserManager;
use common::errors::AppError;
use common::errors::AppError::BizError;
use common::repository_util::Repository;
use common::util::date_util::now;
use crate::result::result;

/// 加入群组请求体
#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GroupJoinDto {
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

pub async fn group_join(dto: web::Json<GroupJoinDto>, auth_header: web::Header<AuthHeader>) -> Result<impl Responder, AppError> {
    let (_agent, valid) = AgentService::get().checksum_request(&*auth_header).await?;
    if !valid {
        return Err(BizError("signature.error".to_string()));
    }

    let redis_user_manager = RedisUserManager::get();
    let now = now();

    let exists = redis_user_manager.find_member(&dto.group_id, &dto.user_id).await?;
    if exists {
        return Ok(web::Json(result()));
    }

    // ✅ 插入成员记录
    let member = GroupMember {
        id: "".to_string(),
        group_id: dto.group_id.clone(),
        user_id: dto.user_id.clone(),
        role: GroupRole::Member,
        alias: dto.alias.clone(),
        mute_until: None,
        mute:false,
        create_time: now,
        update_time: now,
    };
    GroupMemberService::get().dao.insert(&member).await?;
    //添加用户到组
    RedisUserManager::get().add_to_group(&dto.group_id, &dto.user_id).await?;
    //发送消息
    GroupOperationLogService::get().add_log(&dto.group_id, &dto.user_id, None, GroupOperationType::Join).await?;
    Ok(web::Json(result()))
}