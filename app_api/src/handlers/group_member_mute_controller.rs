use crate::result::{result, result_data};
use actix_web::{Responder, post, web, get};
use biz_service::biz_service::agent_service::{AgentService, AuthHeader};
use biz_service::biz_service::group_member_service::GroupMemberService;
use biz_service::biz_service::group_service::GroupService;
use common::errors::AppError;
use common::errors::AppError::BizError;
use common::repository_util::Repository;
use common::util::date_util::now;
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use biz_service::biz_service::mq_group_operation_log_service::GroupOperationLogService;
use biz_service::entitys::mq_group_operation_log::{GroupOperationLog, GroupOperationType};
use common::util::common_utils::as_ref_to_string;


/// 取消禁言成员请求体
#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MuteMemberDto {
    /// 群组 ID
    pub group_id: String,

    /// 用户 ID
    pub user_id: String,

    /// 是否禁言（true=禁言，false=取消禁言）
    pub mute: bool,

    /// 禁言截止时间（毫秒时间戳，Optional）
    pub mute_until: Option<i64>,
}
pub async fn mute_member(dto: web::Json<MuteMemberDto>, auth_header: web::Header<AuthHeader>) -> Result<impl Responder, AppError> {
    let (_agent, valid) = AgentService::get().checksum_request(&*auth_header).await?;
    if !valid {
        return Err(BizError("signature.error".to_string()));
    }
    let mut update_doc=doc! {};
    match dto.mute_until {
        Some(time) => {
            if time < now() {
                return Err(BizError("mute_until.invalid".to_string()));
            }
            update_doc.insert("mute_until", time);
            update_doc.insert("mute", true);
        }
        _ => {}
    }
    if dto.mute {
        update_doc.insert("mute", true);
    }

    GroupMemberService::get().dao.update(doc! {"group_id":&*dto.group_id,"user_id":&*dto.user_id}, update_doc).await?;
    GroupOperationLogService::get().add_log_expired_at(&*dto.group_id, &*dto.user_id, None, GroupOperationType::Mute,dto.mute_until).await?;
    Ok(web::Json(result()))
}
/// 查询群组内被禁言的成员列表
#[get("/group/mute/member/list/{group_id}")]
pub async fn list_muted_members(
    group_id: web::Path<String>
) -> Result<impl Responder, AppError> {
    let now = now();
    let members = GroupMemberService::get()
        .dao
        .query(doc! {
            "group_id": group_id.as_str(),
            "mute": true,
        }).await?;
    Ok(web::Json(result_data(members)))
}

/// 取消群组全体禁言
pub async fn cancel_group_mute(
    group_id: web::Path<String>,
    auth_header: web::Header<AuthHeader>
) -> Result<impl Responder, AppError> {
    let (_agent, valid) = AgentService::get().checksum_request(&*auth_header).await?;
    if !valid {
        return Err(BizError("signature.error".to_string()));
    }
    GroupService::get()
        .dao
        .up_property(group_id.as_str(), "mute", &false)
        .await?;

    let mut log = GroupOperationLog::default();
    log.group_id =as_ref_to_string(&*group_id);
    log.operator_id = "system".to_string();
    log.sync_statue = false;
    log.action= GroupOperationType::UnmuteAll;
    GroupOperationLogService::get().dao.insert(&log).await?;
    Ok(web::Json(result()))
}

/// 添加或移除群组禁言白名单成员的请求体
#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WhiteListUserDto {
    /// 群组 ID
    pub group_id: String,

    /// 用户 ID（要添加/移除白名单的用户）
    pub user_id: String,
}
/// 取消指定群成员禁言
pub async fn cancel_mute_member(
    dto: web::Json<WhiteListUserDto>,
    auth_header: web::Header<AuthHeader>
) -> Result<impl Responder, AppError> {
    let (_agent, valid) = AgentService::get().checksum_request(&*auth_header).await?;
    if !valid {
        return Err(BizError("signature.error".to_string()));
    }
    GroupMemberService::get()
        .dao
        .update(doc! {"group_id":&*dto.group_id,"user_id":&*dto.user_id},
            doc! {
                "mute": false,
                "mute_until": Option::<i64>::None,
                "update_time": now(),
            }
        ).await?;
    
    GroupOperationLogService::get().add_log(&*dto.group_id, &*dto.user_id, None, GroupOperationType::Unmute).await?;
    Ok(web::Json(result()))
}