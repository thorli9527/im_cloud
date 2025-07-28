use crate::handlers::auth::login_handler::*;
use crate::handlers::auth::register_handler::*;
use crate::handlers::auth::reset_password_handler::*;
use actix_web::{get, web, HttpResponse, Responder};
use utoipa::OpenApi;
#[derive(OpenApi)]
#[openapi(
    paths(
        //注册
        auth_register,
        auth_register_verify,
        auth_login,
        //重置密码
        auth_reset_password_send_code,
        auth_reset_password_verify_code,
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
    cfg.service(actix_files::Files::new("/swagger-ui", "./static/swagger-ui").index_file("index.html")).service(openapi_json);
}
#[get("/openapi.json")]
async fn openapi_json() -> impl Responder {
    HttpResponse::Ok().content_type("application/json").body(ApiDoc::openapi().to_json().unwrap())
}
