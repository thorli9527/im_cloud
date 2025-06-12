use crate::result::{ApiResponse, AppState, result, result_data};
use actix_web::{HttpRequest, Responder, post, web};
use biz_service::biz_const::redis_const::CLIENT_TOKEN_KEY;
use biz_service::biz_service::agent_service::{AgentService, build_header};
use biz_service::biz_service::client_service::ClientService;
use biz_service::biz_service::mq_user_action_service::UserActionLogService;
use biz_service::manager::user_manager::RedisUserManager;
use common::errors::AppError;
use common::errors::AppError::BizError;
use common::redis::redis_template::RedisTemplate;
use common::repository_util::Repository;
use common::util::common_utils::{as_ref_to_string, build_uuid};
use common::util::date_util::time_to_str;
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;
use validator::Validate;

pub fn configure(cfg: &mut web::ServiceConfig, state: &web::Data<AppState>) {
    cfg.service(build_user);
    cfg.service(lock);
}
#[derive(Debug, Serialize, Deserialize, ToSchema, Default, Validate)]
#[serde(rename_all = "camelCase")]
struct TokenDto {
    #[validate(length(min = 16, message = "user_id 不能为空，且长度至少为 16"))]
    user_id: String,
    #[validate(length(min = 6, message = "name 不能为空，且长度至少为 6"))]
    name: String,
    avatar_url: Option<String>,
}

#[utoipa::path(
    post,
    path = "/user/build_user",
    params(
        ("appKey" = String, Header, description = "应用 key"),
        ("nonce" = String, Header, description = "随机字符串"),
        ("timestamp" = i64, Header, description = "时间戳"),
        ("signature" = String, Header, description = "签名")
    ),
    request_body = TokenDto,
    responses(
        (status = 200, description = "Hello response", body = ApiResponse<String>)
    )
)]
#[post("/user/build_user")]
pub async fn build_user(dto: web::Json<TokenDto>, req: HttpRequest) -> Result<impl Responder, AppError> {
    let auth_header = build_header(req);
    let agent_service = AgentService::get();
    let (agent, check_state) = agent_service.check_request(auth_header).await?;
    if !check_state {
        return Err(BizError("signature.error".to_string()));
    }

    let user_profile_service = ClientService::get();

    let client = user_profile_service.find_by_user_id(&dto.user_id).await?;
    let user;
    if client.id.is_empty() {
        user = user_profile_service.new_data(agent.id, dto.user_id.clone(), dto.name.clone(), dto.avatar_url.clone()).await?;
    } else {
        user = client;
    }

    let redis_service = RedisTemplate::get();
    let token_id = build_uuid();
    let key = format!("{} {}", CLIENT_TOKEN_KEY, &token_id);
    redis_service.set_key_value(key, user).await?;
    let value = json!({"user_id":dto.user_id,"token":token_id,"avatarUrl":dto.avatar_url});
    Ok(web::Json(result_data(value)))
}
//锁定用户
#[utoipa::path(
    post,
    path = "/user/lock/{user_id}",
    params(
        ("appKey" = String, Header, description = "应用 key"),
        ("nonce" = String, Header, description = "随机字符串"),
        ("timestamp" = i64, Header, description = "时间戳"),
        ("signature" = String, Header, description = "签名")
    ),
    request_body = TokenDto,
    responses(
        (status = 200, description = "Hello response", body = ApiResponse<String>)
    )
)]
#[post("/user/lock/{user_id}")]
pub async fn lock(user_id: web::Path<String>, req: HttpRequest) -> Result<impl Responder, AppError> {
    let auth_header = build_header(req);
    let (agent, check_state) = AgentService::get().check_request(auth_header).await?;
    if !check_state {
        return Err(AppError::Unauthorized("signature.error".to_string()));
    }
    UserActionLogService::get().offline(&*user_id, "系统强制下线", "", Option::None).await?;
    //发送用户lock Mq
    Ok(web::Json(result()))
}
#[utoipa::path(
    post,
    path = "/user/info/{user_id}",
    request_body = TokenDto,
    params(
        ("appKey" = String, Header, description = "应用 key"),
        ("nonce" = String, Header, description = "随机字符串"),
        ("timestamp" = i64, Header, description = "时间戳"),
        ("signature" = String, Header, description = "签名")
    ),
    responses(
        (status = 200, description = "Hello response", body = ApiResponse<String>)
    )
)]
#[post("/user/info/{user_id}")]
pub async fn info(user_id: web::Path<String>, req: HttpRequest) -> Result<impl Responder, AppError> {
    let auth_header = build_header(req);
    let (agent, check_state) = AgentService::get().check_request(auth_header).await?;
    if !check_state {
        return Err(AppError::BizError("signature.error".to_string()));
    }
    let client = ClientService::get().find_by_user_id(&*user_id).await?;
    let value = json!({"user_name":client.name,"avatarUrl":client.avatar_url,"avatarUrl":client.avatar_url,"create_time":time_to_str(client.create_time)});
    Ok(web::Json(result_data(value)))
}
#[derive(Serialize, Deserialize, Debug, Validate, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserInfoDto {
    user_name: String,
    create_time: u64,
    avatar_url: Option<String>,
}
#[derive(Serialize, Deserialize, Debug, Validate, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserEnableDto {
    user_id: String,
    enable: bool,
}

//禁用户户
#[utoipa::path(
    post,
    path = "/user/expire",
    params(
        ("appKey" = String, Header, description = "应用 key"),
        ("nonce" = String, Header, description = "随机字符串"),
        ("timestamp" = i64, Header, description = "时间戳"),
        ("signature" = String, Header, description = "签名")
    ),
    request_body = UserEnableDto,
    responses(
        (status = 200, description = "Hello response", body = ApiResponse<String>)
    )
)]
#[post("/user/expire")]
pub async fn expire(dto: web::Json<UserEnableDto>, req: HttpRequest) -> Result<impl Responder, AppError> {
    let auth_header = build_header(req);
    let (agent, check_state) = AgentService::get().check_request(auth_header).await?;
    if !check_state {
        return Err(BizError("signature.error".to_string()));
    }
    UserActionLogService::get().ban(&*dto.user_id, "system.ban", "", Option::None).await?;
    //发送用户下线MQ
    Ok(web::Json(result()))
}
#[derive(Serialize, Deserialize, Debug, Validate, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RefreshDto {
    user_id: String,
    name: Option<String>,
    avatar_url: Option<String>,
}
#[utoipa::path(
    post,
    path = "/user/refresh",
    params(
        ("appKey" = String, Header, description = "应用 key"),
        ("nonce" = String, Header, description = "随机字符串"),
        ("timestamp" = i64, Header, description = "时间戳"),
        ("signature" = String, Header, description = "签名")
    ),
    request_body = RefreshDto,
    responses(
        (status = 200, description = "Hello response", body = ApiResponse<String>)
    )
)]
#[post("/user/refresh")]
pub async fn refresh(dto: web::Json<RefreshDto>, req: HttpRequest) -> Result<impl Responder, AppError> {
    let auth_header = build_header(req);
    let (agent, check_state) = AgentService::get().check_request(auth_header).await?;
    if !check_state {
        return Err(BizError("signature.error".to_string()));
    }
    let client_service = ClientService::get();
    let client = client_service.find_by_user_id(&*dto.user_id).await?;
    if client.id.is_empty() {
        return Err(BizError("user.not.exist".to_string()));
    }
    let mut up_doc = doc! {};
    if let Some(name) = &dto.name {
        up_doc.insert("name", as_ref_to_string(name));
    }
    if let Some(url) = &dto.avatar_url {
        up_doc.insert("avatar_url", as_ref_to_string(url));
    }
    client_service.dao.update(doc! {"_id":client.id}, up_doc).await?;
    Ok(web::Json(result()))
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserGroupPageDto {
    /// 用户 ID
    pub user_id: String,

    /// 页码（从 1 开始）
    pub page: u64,

    /// 每页大小
    pub size: u64,
}
#[utoipa::path(
    post,
    path = "/user/user_group_page",
    params(
        ("appKey" = String, Header, description = "应用 key"),
        ("nonce" = String, Header, description = "随机字符串"),
        ("timestamp" = i64, Header, description = "时间戳"),
        ("signature" = String, Header, description = "签名")
    ),
    request_body = UserGroupPageDto,
    responses(
        (status = 200, description = "Hello response", body = ApiResponse<String>)
    )
)]
#[post("/user/refresh")]
pub async fn user_group_page(query: web::Json<UserGroupPageDto>, req: HttpRequest) -> Result<impl Responder, AppError> {
    let auth_header = build_header(req);
    let (agent, check_state) = AgentService::get().check_request(auth_header).await?;
    if !check_state {
        return Err(BizError("signature.error".to_string()));
    }

    // 2. 查询
    let group_list = RedisUserManager::get().list_user_groups(&query.user_id, query.page, query.size).await?;

    // 3. 返回
    Ok(web::Json(result_data(group_list)))
}
