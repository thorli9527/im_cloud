mod auth;
mod common_handler;
mod socket_handler;
mod swagger;
mod user_handler;

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    //    swagger::configure(cfg);
    common_handler::configure(cfg);
    socket_handler::configure(cfg);
}
