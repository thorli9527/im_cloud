use crate::handlers::friend_handler::*;
use crate::handlers::group::group_create::*;
use crate::handlers::group::group_dismiss::*;
use crate::handlers::group::group_quit::*;
use crate::handlers::group::group_refresh::*;
use crate::handlers::group::group_transfer::*;
use crate::handlers::group_member::group_member_join::*;
use crate::handlers::group_member::group_member_page::*;
use crate::handlers::group_member::group_member_refresh::*;
use crate::handlers::group_member::group_member_remove::*;
use crate::handlers::user_controller::*;
use crate::handlers::auth_handler::*;

use crate::result::ApiResponse;
use actix_web::{get, web, HttpResponse, Responder};
use biz_service::entitys::agent_entity::AgentInfo;
use common::repository_util::PageResult;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        //用户-管理
        user_create,
        user_generate_token,
        user_change_pass,
        user_set_password,
        user_lock,
        user_un_lock,
        user_refresh,
        user_info,
        user_expire,
        user_login,

        //群-管理
        group_create,
        group_dismiss,
        group_quit,
        group_refresh,
        group_transfer,

        //群-成员-管理
        group_member_join,
        group_member_refresh,
        group_member_remove,
        group_member_page,

        //好友-管理
        friend_add,
        friend_remove,
        friend_check,
        friend_list,
    ),
    components(schemas(
        PageResult<AgentInfo>,
        ApiResponse<String>,
        AgentInfo,
    )),
    tags(
        (name = "im-swagger-api", description = "Example endpoints")
    )
)]
struct ApiDoc;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(openapi_json);
    cfg.service(actix_files::Files::new("/swagger-ui", "./static/swagger-ui").index_file("index.html"))
        .service(openapi_json);
}
#[get("/openapi.json")]
async fn openapi_json() -> impl Responder {
    HttpResponse::Ok().content_type("application/json").body(ApiDoc::openapi().to_json().unwrap())
}
