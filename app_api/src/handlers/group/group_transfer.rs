use crate::handlers::common_handler::status;
use crate::result::{result, ApiResponse};
use actix_web::{post, web, HttpRequest, Responder};
use biz_service::biz_service::agent_service::{build_header, AgentService};
use biz_service::biz_service::group_service::GroupService;
use biz_service::biz_service::mq_group_operation_log_service::GroupOperationLogService;
use biz_service::entitys::mq_group_operation_log::GroupOperationType;
use common::errors::AppError;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(status);
}
/// 转让群组请求体
#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct TransferGroupDto {
    /// 群组 ID
    pub group_id: String,

    /// 新群主用户 ID
    pub new_owner_id: String,
}
#[utoipa::path(
    post,
    path = "/group/transfer",
    request_body = TransferGroupDto,
    summary = "转让群组所有权",
    tag = "群管理",
    params(
        ("appKey" = String, Header, description = "应用 key"),
        ("nonce" = String, Header, description = "随机字符串"),
        ("timestamp" = i64, Header, description = "时间戳"),
        ("signature" = String, Header, description = "签名")
    ),
    responses(
        (status = 200, description = "群组转让成功", body = ApiResponse<String>)
    )
)]
#[post("/group/transfer")]
async fn group_transfer(dto: web::Json<TransferGroupDto>, req: HttpRequest) -> Result<impl Responder, AppError> {
    let auth_header = build_header(req);
    let agent = AgentService::get().check_request(auth_header).await?;
    GroupService::get().transfer_ownership(&dto.group_id, &dto.new_owner_id).await?;
    GroupOperationLogService::get()
        .add_log(&agent.id, &*dto.group_id, &*dto.new_owner_id, None, GroupOperationType::Transfer)
        .await?;
    Ok(web::Json(result()))
}
