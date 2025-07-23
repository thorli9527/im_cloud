use crate::result::{result, result_data, result_error};
use actix_web::{Responder, post, web};
use anyhow::anyhow;
use biz_service::biz_service::user_service::UserService;
use biz_service::entitys::user_entity::UserInfoEntity;
use common::config::AppConfig;
use common::errors::AppError;
use common::models::property_value::PropertyValue;
use common::repository_util::{OrderType, Repository};
use common::util::common_utils::{build_md5_with_key, build_uuid};
use common::util::date_util::now;
use mongo_macro::QueryFilter;
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;
use validator::Validate;
use web::Json;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(user_list);
    cfg.service(user_add);
}
#[derive(QueryFilter, Serialize, Deserialize, Debug, Validate, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserInfoQueryDto {
    #[query(eq)]
    pub user_name: Option<String>,
    #[query(gt, field = "_id")]
    pub max_id: Option<String>,
    pub page_size: i64,
}

#[post("/user/list")]
pub async fn user_list(dto: Json<UserInfoQueryDto>) -> Result<impl Responder, AppError> {
    let page_result = UserService::get()
        .dao
        .query_by_page(dto.to_query_doc(), dto.page_size, Some(OrderType::Asc), "_id")
        .await?;
    Ok(Json(result_data(page_result)))
}

#[derive(Serialize, Deserialize, Debug, Validate, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserAddDto {
    #[validate(length(min = 5, message = "用户名太短"))]
    pub user_name: Option<String>,
    #[validate(length(min = 5, message = "密码太短"))]
    pub password: Option<String>,
    pub is_admin: bool,
}
#[post("/user/add")]
pub async fn user_add(dto: Json<UserAddDto>) -> Result<impl Responder, AppError> {
    if let Err(e) = &dto.validate() {
        return Ok(result_error(e.to_string()));
    }
    let user_service = UserService::get();
    let user = user_service.dao.find_one(doc! {"user_name": &dto.user_name}).await?;
    if user.is_some() {
        return Ok(result_error("用户已存在"));
    }
    let sys_config = AppConfig::get().clone().sys.clone().unwrap();
    let md5_key = &sys_config.md5_key.unwrap();
    let user_summery = dto.clone();
    let now = now() as u64;
    let user = UserInfoEntity {
        id: build_uuid(),
        user_name: user_summery.user_name.unwrap(),
        password: build_md5_with_key(&user_summery.password.unwrap(), md5_key),
        status: true,
        is_admin: false,
        create_time: now,
        update_time: now,
    };
    user_service.dao.insert(&user).await?;
    Ok(result())
}

#[post("/user/change/{uid}")]
pub async fn property_change(
    uid: web::Path<String>,
    body: Json<PropertyValue>,
) -> Result<impl Responder, AppError> {
    let property = body.into_inner();
    let allowed_fields = ["status", "is_admin"];

    if !allowed_fields.contains(&property.property_name.as_str()) {
        return Ok(web::Json(json!({
            "code": 500,
            "message": format!("字段 '{}' 不支持修改", property.property_name)
        })));
    }

    let user_service = UserService::get();
    let user =
        user_service.dao.find_by_id(&uid).await.map_err(|e| anyhow!("用户查询失败: {}", e))?;

    if user.is_none() {
        return Ok(web::Json(json!({
            "code": 500,
            "message": "字段暂不支持修改"
        })));
    }

    let mut user = user.unwrap();
    let new_value = property.value.trim();
    match property.property_name.as_str() {
        "status" => {
            user.status =
                new_value.parse::<bool>().map_err(|_| anyhow!("status 字段应为 true 或 false"))?;
        }
        "is_admin" => {
            user.is_admin = new_value
                .parse::<bool>()
                .map_err(|_| anyhow!("is_admin 字段应为 true 或 false"))?;
        }
        _ => {
            return Ok(web::Json(json!({
              "code": 500,
                "message": "字段暂不支持修改"
            })));
        }
    }

    user.update_time = now() as u64;

    user_service.dao.save(&user).await.expect("更新用户信息失败");

    return Ok(web::Json(json!({
        "code": 0,
        "message": "字段暂不支持修改"
    })));
}
#[post("/user/del/{user_id}")]
pub async fn user_del(user_id: web::Path<String>) -> Result<impl Responder, AppError> {
    UserService::get().dao.delete_by_id(&user_id).await?;
    Ok(result())
}

#[derive(Serialize, Deserialize, Debug, Validate, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ResetPassword {
    pub user_id: String,
    #[validate(length(min = 5, message = "密码太短"))]
    pub password: Option<String>,
}

#[post("/user/reset/password")]
pub async fn reset_password(dto: Json<ResetPassword>) -> Result<impl Responder, AppError> {
    match &dto.validate() {
        Ok(_) => {
            let md5_key = AppConfig::get().sys.clone().unwrap().md5_key.unwrap();
            let password = build_md5_with_key(&md5_key, &dto.password.as_ref().unwrap());
            UserService::get().dao.up_property(&dto.user_id, "status", password).await?;
            Ok(result())
        }
        Err(e) => return Ok(result_error(e.to_string())),
    }
}

#[derive(Serialize, Deserialize, Debug, Validate, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserPassChange {
    pub user_id: String,

    #[validate(length(min = 5, message = "原密码太短"))]
    pub old_password: Option<String>,

    #[validate(length(min = 5, message = "新密码太短"))]
    pub new_password: Option<String>,
}
#[post("/user/change/pass")]
pub async fn user_change_pass(dto: Json<UserPassChange>) -> Result<impl Responder, AppError> {
    // Step 1: 校验字段合法性
    dto.validate().map_err(|e| anyhow!(e.to_string()))?;

    let user_id = &dto.user_id;
    let old_password = dto.old_password.as_ref().unwrap();
    let new_password = dto.new_password.as_ref().unwrap();

    let user_service = UserService::get();

    // Step 2: 查询用户信息
    let user = user_service
        .dao
        .find_by_id(user_id)
        .await
        .map_err(|e| anyhow!(format!("用户查询失败: {}", e)))?;

    if user.is_none() {
        return Ok(result_error("用户不存在"));
    }
    let md5_key = AppConfig::get().clone().sys.clone().unwrap().md5_key.unwrap();
    let user = user.unwrap();

    // Step 3: 验证原密码
    let input_encrypted = build_md5_with_key(old_password, &md5_key);
    if user.password != input_encrypted {
        return Ok(result_error("原密码不正确"));
    }

    // Step 4: 构建新密码并更新
    let new_encrypted = build_md5_with_key(new_password, &md5_key);
    user_service.dao.up_property(user_id, "password", new_encrypted).await?;

    Ok(result())
}
