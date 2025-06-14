use crate::handlers::common_handler::*;
use crate::handlers::user_contoller::*;
use crate::result::ApiResponse;
use actix_web::{get, web, HttpResponse, Responder};
use biz_service::entitys::agent_entity::AgentInfo;
use common::repository_util::PageResult;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        status,
        build_user,
    ),
    components(schemas(
        PageResult<AgentInfo>,
        ApiResponse<String>,
        AgentInfo,
    )),
    tags(
        (name = "登录", description = "Example endpoints")
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
