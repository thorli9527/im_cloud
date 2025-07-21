mod common_handler;
mod swagger;
mod socket_handler;

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    swagger::configure(cfg);
    common_handler::configure(cfg);
}
