use crate::result::{result, result_data, ApiResponse};
use actix_web::{post, web, HttpRequest, Responder};
use biz_service::biz_service::agent_service::{build_header, AgentService};
use biz_service::manager::user_manager_core::{UserManager, UserManagerOpt};
use biz_service::protocol::msg::friend::FriendSourceType;
use common::errors::AppError;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(friend_add);
    cfg.service(friend_remove);
    cfg.service(friend_check);
    cfg.service(friend_list);
}

#[utoipa::path(
    post,
    path = "/user/friend/add",
    tag = "好友-管理",
    summary = "添加好友",
    request_body = FriendOpDto,
    responses((status = 200, description = "成功", body = ApiResponse<String>))
)]
#[post("/user/friend/add")]
async fn friend_add(dto: web::Json<FriendOpDto>, req: HttpRequest) -> Result<impl Responder, AppError> {
    dto.validate()?;
    let auth = build_header(req);
    UserManager::get().add_friend( &dto.uid, &dto.friend_id, dto.nickname.clone(), &dto.source_type, dto.remark.clone()).await?;
    Ok(web::Json(result()))
}

#[utoipa::path(
    post,
    path = "/user/friend/remove",
     tag = "好友-管理",
    summary = "删除好友",
    request_body = FriendOpDto,
    responses((status = 200, description = "成功", body = ApiResponse<String>))
)]
#[post("/user/friend/remove")]
async fn friend_remove(dto: web::Json<FriendOpDto>, req: HttpRequest) -> Result<impl Responder, AppError> {
    let auth = build_header(req);
    let agent = AgentService::get().check_request(auth).await?;
    UserManager::get().remove_friend( &dto.uid, &dto.friend_id).await?;
    Ok(web::Json(result()))
}

#[utoipa::path(
    post,
    path = "/user/friend/check",
       tag = "好友-管理",
    summary = "检查是否为好友",
    request_body = FriendCheckDto,
    responses((status = 200, description = "true/false", body = ApiResponse<bool>))
)]
#[post("/user/friend/check")]
async fn friend_check(dto: web::Json<FriendCheckDto>, req: HttpRequest) -> Result<impl Responder, AppError> {
    let auth = build_header(req);
    let result = UserManager::get().is_friend( &dto.uid, &dto.friend_id).await?;
    Ok(web::Json(result_data(result)))
}

#[utoipa::path(
    post,
    path = "/user/friend/list",
   tag = "好友-管理",
    summary = "获取好友列表",
    request_body = FriendListDto,
    responses((status = 200, description = "好友 ID 列表", body = ApiResponse<Vec<String>>))
)]
#[post("/user/friend/list")]
async fn friend_list(dto: web::Json<FriendListDto>, req: HttpRequest) -> Result<impl Responder, AppError> {
    let auth = build_header(req);
    let list = UserManager::get().get_friends( &dto.uid).await?;
    Ok(web::Json(result_data(list)))
}

#[derive(Debug, Deserialize, Serialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct FriendOpDto {
    #[validate(length(min = 2, max = 64, message = "用户ID不能为空，长度为2~64"))]
    pub uid: String,

    #[validate(length(min = 2, max = 64, message = "好友ID不能为空，长度为2~64"))]
    pub friend_id: String,

    #[validate(length(min = 0, max = 32, message = "昵称不能超过32个字符"))]
    pub nickname: Option<String>,

    #[validate(length(min = 0, max = 64, message = "备注不能超过64个字符"))]
    pub remark: Option<String>,

    pub source_type: FriendSourceType,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct FriendCheckDto {
    pub uid: String,
    pub friend_id: String,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct FriendListDto {
    pub uid: String,
}
