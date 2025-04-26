use actix_web::{web, Responder};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use biz_service::biz_service::agent_service::AuthHeader;
use biz_service::manager::user_manager::RedisUserManager;
use common::errors::AppError;
use common::errors::AppError::BizError;
use crate::result::result_data;

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

pub async fn group_member_page(
    query: web::Json<GroupMemberPageDto>,
    auth_header: web::Header<AuthHeader>,
) -> Result<impl Responder, AppError> {
    // 1. 签名校验
    let (_agent, valid) = biz_service::biz_service::agent_service::AgentService::get()
        .checksum_request(&*auth_header)
        .await?;
    if !valid {
        return Err(BizError("signature.error".to_string()));
    }

    // 2. 分页查询
    let members = RedisUserManager::get()
        .list_group_members(&query.group_id, query.page, query.size)
        .await?;

    // 3. 返回
    Ok(web::Json(result_data(members)))
}
