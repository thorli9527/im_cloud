use crate::result::{result, result_data, ApiResponse};
use actix_web::{post, web, HttpRequest, Responder};
use biz_service::biz_const::redis_const::CLIENT_TOKEN_KEY;
use biz_service::biz_service::agent_service::{build_header, AgentService};
use biz_service::biz_service::client_service::ClientService;
use biz_service::entitys::client_entity::ClientInfo;
use biz_service::manager::group_manager_core::{GroupManager, GroupManagerOpt};
use biz_service::manager::user_manager_core::{UserManager, UserManagerOpt};
use common::config::AppConfig;
use common::errors::AppError;
use common::errors::AppError::BizError;
use common::redis::redis_template::RedisTemplate;
use common::redis::redis_template::ValueOps;
use common::repository_util::Repository;
use common::util::common_utils::{as_ref_to_string, build_md5_with_key, build_uuid};
use common::util::date_util::time_to_str;
use deadpool_redis::redis::AsyncCommands;
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;
use validator::Validate;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(user_create);
    cfg.service(user_lock);
    cfg.service(user_info);
    cfg.service(user_refresh);
    cfg.service(user_expire);
    cfg.service(user_generate_token);
    cfg.service(user_set_password);
}
#[derive(Debug, Serialize, Deserialize, ToSchema, Default, Validate)]
#[serde(rename_all = "camelCase")]
struct UserCreateDto {
    #[validate(length(min = 2, max = 32, message = "uid 不能为空，且长度至少为2位 最大32位"))]
    uid: String,
    #[validate(length(min = 2, max = 32, message = "name 不能为空，且长度至少为 2位,最大为 32位"))]
    name: String,
    avatar: Option<String>,
    username: Option<String>,
    password: Option<String>,
}

#[utoipa::path(
    post,
    path = "/user/create",
    tag = "用户管理",
    summary = "创建用户",

    params(
        ("appKey" = String, Header, description = "应用 key"),
        ("nonce" = String, Header, description = "随机字符串"),
        ("timestamp" = i64, Header, description = "时间戳"),
        ("signature" = String, Header, description = "签名")
    ),
    request_body = UserCreateDto,
    responses(
        (status = 200, description = "Hello response", body = ApiResponse<String>)
    )
)]
#[post("/user/create")]
async fn user_create(dto: web::Json<UserCreateDto>, req: HttpRequest) -> Result<impl Responder, AppError> {
    dto.validate()?;
    let auth_header = build_header(req);
    let agent_service = AgentService::get();
    let agent = agent_service.check_request(auth_header.clone()).await?;
    let client_service = ClientService::get();
    let string = &dto.uid.clone();
    let user_manager = UserManager::get();
    let user_option: Option<ClientInfo> = user_manager.get_user_info(&agent.id, string).await?;
    let redis_template = RedisTemplate::get();
    if user_option.is_some() {
        let user: ClientInfo = user_option.unwrap();
        let token_id = build_uuid();
        let key = format!("{}{}", CLIENT_TOKEN_KEY, &token_id);
        let value_option = redis_template.ops_for_value();
        let _ = value_option.set(&key, &user.clone(), Some(30 * 60)).await?;
        let value = json!({"uid":user.uid,"token":token_id,"avatar":user.avatar});
        return Ok(web::Json(result_data(value)));
    }

    let user = client_service.new_data(agent.id.clone(), &dto.uid.clone(), dto.name.clone(), dto.avatar.clone(),dto.username.clone(),dto.password.clone()).await?;
    let token_key = user_manager.build_token(&user.agent_id, &user.uid, &auth_header.unwrap().device_type).await?;
    user_manager.sync_user(user).await?;
    let value = json!({"token":token_key,"avatar":dto.avatar});
    Ok(web::Json(result_data(value)))
}



#[derive(Debug, Serialize, Deserialize, ToSchema, Default, Validate)]
#[serde(rename_all = "camelCase")]
struct TokenDto{
    uid: String,
}
#[utoipa::path(
    post,
    path = "/user/token/generate",
    tag = "用户管理",
    summary = "生成用户 Token",
    params(
        ("appKey" = String, Header, description = "应用 key"),
        ("nonce" = String, Header, description = "随机字符串"),
        ("timestamp" = i64, Header, description = "时间戳"),
        ("signature" = String, Header, description = "签名")
    ),
    request_body = TokenDto,
    responses(
        (status = 200, description = "Token生成成功", body = ApiResponse<String>)
    )
)]
#[post("/user/token/generate")]
async fn user_generate_token(dto: web::Json<TokenDto>, req: HttpRequest) -> Result<impl Responder, AppError> {
    dto.validate()?;
    let auth_header = build_header(req);
    let agent = AgentService::get().check_request(auth_header.clone()).await?;
    let user_manager = UserManager::get();
    // 查询用户是否存在
    let user = match user_manager.get_user_info(&agent.id, &dto.uid).await? {
        Some(u) => u,
        None => {
           return Err(AppError::BizError("User not found".to_string()));
        }
    };

    // 生成 token 并保存到 redis
    let token_key = user_manager
        .build_token(&user.agent_id, &user.uid, &auth_header.unwrap().device_type)
        .await?;

    let value = json!({
        "token": token_key,
        "avatar": user.avatar
    });
    
    //增加下线通知 待用户用新的token上
    
    Ok(web::Json(result_data(value)))
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
struct SetPasswordDto {
    #[validate(length(min = 2, message = "uid 不能为空"))]
    uid: String,

    #[validate(length(min = 6, message = "密码长度至少为 6 个字符"))]
    password: String,
}
#[utoipa::path(
    post,
    path = "/user/set_password",
    request_body = SetPasswordDto,
    responses(
        (status = 200, description = "Hello response", body = ApiResponse<String>)
    ),
    tag = "用户管理",
    operation_id = "setUserPassword",
    security(
        ("BearerAuth" = [])
    )
)]
#[post("/user/set_password")]
async fn user_set_password(dto: web::Json<SetPasswordDto>, req: HttpRequest) -> Result<impl Responder, AppError> {
    dto.validate()?;
    let auth_header = build_header(req);
    let agent = AgentService::get().check_request(auth_header).await?;
    let user_manager = UserManager::get();
    let client_service = ClientService::get();
    let app_config = AppConfig::get();

    let client_opt = user_manager.get_user_info(&agent.id, &dto.uid).await?;
    let client = match client_opt {
        Some(c) => c,
        None => return Err(BizError("用户不存在".into()).into()),
    };

    let hashed_password = build_md5_with_key(&dto.password, &app_config.get_sys().md5_key);
    client_service
        .dao
        .update(doc! { "_id": &client.id }, doc! { "password": &hashed_password })
        .await?;
    user_manager.clear_tokens_by_user(&agent.id, &dto.uid).await?;
    Ok(web::Json(result()))
}


//锁定用户
#[utoipa::path(
    post,
    path = "/user/lock/{user_id}",
    tag = "用户管理",
    summary = "锁定用户",
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
async fn user_lock(user_id: web::Path<String>, req: HttpRequest) -> Result<impl Responder, AppError> {
    let auth_header = build_header(req);
    let agent = AgentService::get().check_request(auth_header).await?;
    let action_log_service = UserActionLogService::get();
    action_log_service.lock(&agent.id, &*user_id, "系统强制锁定", "", Option::None).await?;
    //发送用户lock Mq
    Ok(web::Json(result()))
}

#[utoipa::path(
    post,
    path = "/user/user_un_lock/{user_id}",
    tag = "用户管理",
    summary = "锁定用户",
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
#[post("/user/user_un_lock/{user_id}")]
async fn user_un_lock(user_id: web::Path<String>, req: HttpRequest) -> Result<impl Responder, AppError> {
    if user_id.is_empty() {
        return Err(BizError("user.id.empty".to_string()).into());
    }
    let user_id = user_id.into_inner();
    let auth_header = build_header(req);
    let agent = AgentService::get().check_request(auth_header).await?;
    let action_log_service = UserActionLogService::get();
    action_log_service.un_block(&agent.id, &user_id, "系统强制锁定", "", Option::None).await?;
    //发送用户lock Mq
    Ok(web::Json(result()))
}
#[utoipa::path(
    post,
    path = "/user/info/{user_id}",
    request_body = TokenDto,
    tag = "用户管理",
    summary = "获取用户信息",
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
async fn user_info(user_id: web::Path<String>, req: HttpRequest) -> Result<impl Responder, AppError> {
    let auth_header = build_header(req);
    let agent = AgentService::get().check_request(auth_header).await?;
    let user_manager = UserManager::get();
    let client_option = user_manager.get_user_info(&agent.id, user_id.as_ref()).await?;
    if client_option.is_none() {
        return Err(BizError("user.not.exist".to_string()).into());
    }
    let client = client_option.unwrap();
    let value = json!({"user_name":client.name,"avatar":client.avatar,"create_time":time_to_str(client.create_time)});
    Ok(web::Json(result_data(value)))
}
#[derive(Serialize, Deserialize, Debug, Validate, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
struct UserInfoDto {
    user_name: String,
    create_time: u64,
    avatar_url: Option<String>,
}
#[derive(Serialize, Deserialize, Debug, Validate, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
struct UserEnableDto {
    uid: String,
    enable: bool,
}

//禁用户户
#[utoipa::path(
    post,
    path = "/user/expire",
    summary = "禁用用户",
    tag = "用户管理",
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
async fn user_expire(dto: web::Json<UserEnableDto>, req: HttpRequest) -> Result<impl Responder, AppError> {
    let auth_header = build_header(req);
    let agent = AgentService::get().check_request(auth_header).await?;
    UserActionLogService::get().ban(&agent.id, &dto.uid.clone(), "system.ban", "执行失败", Option::None).await?;
    //发送用户下线MQ
    Ok(web::Json(result()))
}
#[derive(Serialize, Deserialize, Debug, Validate, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
struct RefreshDto {
    uid: String,
    name: Option<String>,
    avatar: Option<String>,
}
#[utoipa::path(
    post,
    path = "/user/refresh",
    summary = "刷新用户信息",
    tag = "用户管理",
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
async fn user_refresh(dto: web::Json<RefreshDto>, req: HttpRequest) -> Result<impl Responder, AppError> {
    let auth_header = build_header(req);
    let agent = AgentService::get().check_request(auth_header).await?;
    let client_service = ClientService::get();
    let user_manager = UserManager::get();
    let client = user_manager.get_user_info(&agent.id, &dto.uid).await?;
    if client.is_none() {
        return Err(BizError("user.not.exist".to_string()));
    }
    let mut client = client.unwrap();
    if client.agent_id != agent.id {
        return Err(BizError("user.not.exist".to_string()));
    }
    let mut up_doc = doc! {};
    if let Some(name) = &dto.name {
        up_doc.insert("name", as_ref_to_string(name));
        client.name = name.to_string();
    }
    if let Some(url) = &dto.avatar {
        up_doc.insert("avatar_url", as_ref_to_string(url));
    }
    client.avatar = dto.avatar.clone();
    client_service.dao.update(doc! {"_id":client.id.clone()}, up_doc).await?;
    user_manager.sync_user(client.clone()).await?;
    Ok(web::Json(result()))
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct UserGroupPageDto {
    /// 用户 ID
    pub uid: String,

    /// 页码（从 1 开始）
    pub page: u64,

    /// 每页大小
    pub size: u64,
}
#[utoipa::path(
    post,
    path = "/user/group/page",
    tag = "用户管理",
    summary = "拉取用户群组分页查询",
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
#[post("/user/group/page")]
async fn user_group_page(query: web::Json<UserGroupPageDto>, req: HttpRequest) -> Result<impl Responder, AppError> {
    let auth_header = build_header(req);
    let agent = AgentService::get().check_request(auth_header).await?;

    // 2. 查询
    let group_list = GroupManager::get().get_group_members_by_page(&query.uid, query.page as usize, query.size as usize).await?;

    // 3. 返回
    Ok(web::Json(result_data(group_list)))
}
