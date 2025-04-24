use std::clone;
use crate::result::{result, result_data};
use actix_web::{web, Responder};
use biz_service::biz_const::redis_const::CLIENT_TOKEN_KEY;
use biz_service::biz_service::agent_service::{AgentService, AuthHeader};
use biz_service::biz_service::client_service::ClientService;
use biz_service::biz_service::mq_user_action_service::UserActionLogService;
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

#[derive(Debug, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "camelCase")]
struct TokenDto {
    user_id: String,
    name: String,
    avatar_url: Option<String>,
}

pub async fn build_user(dto: web::Json<TokenDto>, auth_header: web::Header<AuthHeader>) -> Result<impl Responder, AppError> {
    let agent_service = AgentService::get();
    let agent = agent_service.find_by_app_key(&auth_header.app_key).await?;
    let (agent,check_state) = agent_service.checksum_request(&*auth_header).await?;
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
pub async fn lock(user_id: web::Path<String>, auth_header: web::Header<AuthHeader>) -> Result<impl Responder, AppError> {
    let (agent,check_state) = AgentService::get().checksum_request(&*auth_header).await?;
    if !check_state {
        return Err(BizError("signature.error".to_string()));
    }
    UserActionLogService::get().offline(&*user_id, "系统强制下线", "", Option::None).await?;
    //发送用户lock Mq
     Ok(web::Json(result()))
}

pub async fn info(user_id: web::Path<String>, auth_header: web::Header<AuthHeader>) -> Result<impl Responder, AppError> {
    let (agent,check_state) = AgentService::get().checksum_request(&*auth_header).await?;
    if !check_state {
        return Err(BizError("signature.error".to_string()));
    }
    let client = ClientService::get().find_by_user_id(&*user_id).await?;
    let value = json!({"userName":client.name,"avatarUrl":client.avatar_url,"avatarUrl":client.avatar_url,"createTime":time_to_str(client.create_time)});
     Ok(web::Json(result_data(value)))
}

pub struct UserEnableDto {
    user_id: String,
    enable: bool,
}

//禁用户户
pub async fn expire(dto: web::Json<UserEnableDto>, auth_header: web::Header<AuthHeader>) -> Result<impl Responder, AppError> {
    let (agent,check_state) = AgentService::get().checksum_request(&*auth_header).await?;
    if !check_state {
        return Err(BizError("signature.error".to_string()));
    }
    UserActionLogService::get().ban(&*dto.user_id, "system.ban", "", Option::None).await?;
    //发送用户下线MQ
    Ok(web::Json(result()))
}

struct RefreshDto{
    user_id:String,
    name:Option<String>,
    avatar_url:Option<String>

}
pub async fn refresh(dto: web::Json<RefreshDto>, auth_header: web::Header<AuthHeader>) -> Result<impl Responder, AppError> {
    let (agent,check_state) = AgentService::get().checksum_request(&*auth_header).await?;
    if !check_state {
        return Err(BizError("signature.error".to_string()));
    }
    let client_service = ClientService::get();
    let client = client_service.find_by_user_id(&*dto.user_id).await?;
    if client.id.is_empty() {
        return Err(BizError("user.not.exist".to_string()));
    }
    let mut up_doc = doc! {};
    if let Some(name)=&dto.name{
        up_doc.insert("name",as_ref_to_string(name));
    }
    if let Some(url) = &dto.avatar_url {
        up_doc.insert("avatar_url", as_ref_to_string(url));
    }
    client_service.dao.update(doc! {"_id":client.id}, up_doc).await?;
    Ok(web::Json(result()))
}
