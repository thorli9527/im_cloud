use actix_web::{post, web, HttpRequest, Responder};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;
use validator::Validate;
use biz_service::biz_service::agent_service::{build_header, AgentService};
use biz_service::manager::user_manager_core::{UserManager, UserManagerOpt};
use common::config::AppConfig;
use common::errors::AppError;
use common::errors::AppError::BizError;
use common::util::common_utils::build_md5_with_key;
use crate::result::{result_data, ApiResponse};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(user_login);
}
#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
struct LoginDto {
    #[validate(length(min = 2, message = "uid 不能为空"))]
    uid: String,

    #[validate(length(min = 6, message = "密码不能为空且长度必须大于6位"))]
    password: String,
}


#[utoipa::path(
    post,
    path = "/user/login",
    tag = "用户管理",
    summary = "用户登录",
    request_body = LoginDto,
    responses(
        (status = 200, description = "登录成功", body = ApiResponse<String>)
    )
)]
#[post("/user/login")]
async fn user_login(dto: web::Json<LoginDto>, req: HttpRequest) -> Result<impl Responder, AppError> {
    dto.validate()?;
    let auth_header = build_header(req);
    let agent = AgentService::get().check_request(auth_header.clone()).await?;
    let user_manager = UserManager::get();
    let app_config = AppConfig::get();

    // 查询用户
    let client_opt = user_manager.get_user_info(&agent.id, &dto.uid).await?;
    let client = match client_opt {
        Some(c) => c,
        None => return Err(BizError("用户不存在".into()).into()),
    };

    // 校验密码
    if let Some(stored_password) = &client.password {
        let input_hash = build_md5_with_key(&dto.password, &app_config.sys.md5_key);
        let stored_hash = build_md5_with_key(stored_password, &app_config.sys.md5_key); // 注意：密码存储结构可能已加密

        if input_hash != stored_hash {
            return Err(BizError("密码错误".into()).into());
        }
    } else {
        return Err(BizError("未设置密码".into()).into());
    }

    // 生成 token
    let token = user_manager
        .build_token(&agent.id, &dto.uid, auth_header.unwrap().device_type)
        .await?;

    let value = json!({
        "token": token,
        "avatar": client.avatar
    });

    Ok(web::Json(result_data(value)))
}