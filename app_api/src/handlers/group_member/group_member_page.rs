use crate::result::{result_data, AppState};
use actix_web::{web, HttpRequest, Responder};
use biz_service::biz_service::agent_service::{build_header, AgentService};
use biz_service::manager::user_manager::RedisUserManager;
use common::errors::AppError;
use common::errors::AppError::BizError;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
pub fn configure(cfg: &mut web::ServiceConfig, state: &web::Data<AppState>) {

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

pub async fn group_member_page(
    query: web::Json<GroupMemberPageDto>,
    req: HttpRequest) -> Result<impl Responder, AppError> {
    let auth_header = build_header(req);
    let (agent, check_state) = AgentService::get().check_request(auth_header).await?;
    if !check_state {
        return Err(BizError("signature.error".to_string()));
    }

    // 2. 分页查询
    let members = RedisUserManager::get()
        .list_group_members(&query.group_id, query.page, query.size)
        .await?;

    // 3. 返回
    Ok(web::Json(result_data(members)))
}
