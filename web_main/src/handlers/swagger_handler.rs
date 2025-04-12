use crate::result::AppState;
use actix_web::{get, web, HttpResponse, Responder};
use utoipa::{OpenApi, ToSchema};

pub fn configure(cfg: &mut web::ServiceConfig, state: web::Data<AppState>) {
    cfg.service(actix_files::Files::new("/swagger-ui", "./static/swagger-ui").index_file("index.html"))
        .service(openapi_json);
}
#[derive(OpenApi)]
#[openapi(
    // paths(hello),
    // components(schemas(HelloResponse)),
    // tags(
    //     (name = "Example", description = "Example endpoints")
    // )
)]
struct ApiDoc;

#[get("/openapi.json")]
async fn openapi_json() -> impl Responder {
    HttpResponse::Ok()
        .content_type("application/json")
        .body(ApiDoc::openapi().to_json().unwrap())
}