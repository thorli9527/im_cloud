use crate::result::ApiResponse;
use actix_web::{get, web, HttpResponse, Responder};
use actix_web::web::service;
use common::repository_util::PageResult;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
       
    ),
    components(schemas(
    )),
    tags(
        (name = "im-swagger-api", description = "Example endpoints")
    )
)]
struct ApiDoc;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(openapi_json);
}
#[get("/openapi.json")]
async fn openapi_json() -> impl Responder {
    HttpResponse::Ok().content_type("application/json").body(ApiDoc::openapi().to_json().unwrap())
}
