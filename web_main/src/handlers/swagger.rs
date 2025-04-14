use crate::handlers::common_handler::__path_status;
use crate::handlers::*;
use crate::result::ResultResponse;
use actix_web::{HttpResponse, Responder, get};
use biz_service::entitys::user_entity::*;
use mongodb::bson::oid::ObjectId;
use utoipa::{OpenApi, ToSchema, openapi};
#[derive(OpenApi)]
#[openapi(
    paths(
        auth_login,
        status
    ),
    components(schemas(
        LoginInfoDto,
        ResultResponse<String>,
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
