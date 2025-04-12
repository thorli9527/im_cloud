use actix_web::{get, web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(hello);
}
#[derive(Serialize, Deserialize,ToSchema)]
pub struct HelloResponse {
    message: String,
}

#[utoipa::path(
    get,
    path = "/hello",
    responses(
        (status = 200, description = "Hello response", body = HelloResponse)
    )
)]
#[get("/hello")]
pub async fn hello() -> impl Responder {
    HttpResponse::Ok().json(HelloResponse {
        message: "Hello from Actix + Utoipa!".to_string(),
    })
}
