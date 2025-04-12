use crate::handlers::*;
use actix_web::{App, HttpResponse, HttpServer, Responder, get};
use serde::{Deserialize, Serialize};
use utoipa::{OpenApi, ToSchema};
#[derive(OpenApi)]
#[openapi(
    paths(
        hello,
        auth_login
    ),
    components(schemas(
        HelloResponse,
        LoginInfoDto
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
