use crate::handlers::common_handler::status;
use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(status);
}
