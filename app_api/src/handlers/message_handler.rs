use crate::result::result;
use actix_web::web::ServiceConfig;
use actix_web::{HttpRequest, HttpResponse, Responder, post, web};
use biz_service::biz_service::agent_service::{AgentService, build_header};
use biz_service::biz_service::mq_message_group_service::GroupMessageService;
use biz_service::biz_service::mq_message_user_service::UserMessageService;
use biz_service::entitys::mq_message_info::{MessageSegment, ChatTargetType, Segment, SegmentDto, UserMessage};
use common::errors::AppError;
use common::repository_util::Repository;
use common::util::common_utils::{build_snow_id, build_uuid};
use common::util::date_util::now;
use mongodb::bson::uuid;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;
use validator::Validate;

pub fn configure(cfg: &mut web::ServiceConfig) {}

/// 消息发送 DTO
#[derive(Debug, Deserialize, Serialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct MessageSendDto {
    #[validate(length(min = 16, message = "from 不能为空，且长度至少为 16"))]
    /// 发送者
    pub from: String,
    /// 0: 单聊，1: 群聊
    pub target_type: ChatTargetType,
    #[validate(length(min = 16, message = "to 不能为空，且长度至少为 16"))]
    /// 接收者 accid 或 tid
    pub to: String,
    /// 消息复合内容（支持结构化消息段）
    #[validate(length(min = 1, message = "消息内容不能为空"))]
    pub content: Vec<SegmentDto>,
}

#[utoipa::path(
    post,
    path = "/message/send",
    request_body = MessageSendDto,
    responses(
        (status = 200, description = "消息发送成功", body = HashMap<String, String>)
    )
)]
pub async fn send_message(dto: web::Json<MessageSendDto>, req: HttpRequest) -> Result<impl Responder, AppError> {
    // 验证参数合法性
    dto.validate().map_err(|e| AppError::BizError(format!("验证错误: {:?}", e)))?;

    // 获取认证信息
    let auth_header = build_header(req);
    let agent = AgentService::get().check_request(auth_header).await?;

    // 单聊
    if dto.target_type == ChatTargetType::Single {
        let user_message_service = UserMessageService::get();
        user_message_service.send_user_message(&agent.id, &dto.from, &dto.to, &dto.content).await?;
        return Ok(web::Json(result()));
    }
    // 群聊
    if dto.target_type == ChatTargetType::Group {
        let group_message_service = GroupMessageService::get();
        group_message_service.send_group_message(&agent.id, &dto.from, &dto.to, &dto.content).await?;
        return Ok(web::Json(result()));
    }
    return Err(AppError::BizError("暂不支持消息发送".into()));
}
