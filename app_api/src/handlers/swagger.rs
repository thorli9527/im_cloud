use crate::handlers::common_handler::*;
use crate::handlers::user_contoller::*;
use crate::handlers::group::group_create::*;
use crate::handlers::group::group_dismiss::*;
use crate::handlers::group::group_quit::*;
use crate::handlers::group::group_refresh::*;
use crate::handlers::group::group_transfer::*;

use crate::result::ApiResponse;
use actix_web::{get, web, HttpResponse, Responder};
use biz_service::entitys::agent_entity::AgentInfo;
use common::repository_util::PageResult;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        user_create,
        user_lock,
        user_un_lock,
        user_refresh,
        user_info,
        user_expire,

        //群管理
        group_create,
        group_dismiss,
        group_quit,
        group_refresh,
        group_transfer,
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
