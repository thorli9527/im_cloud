use actix_web::web;

pub mod auth_handler;
pub mod common_handler;
pub mod swagger;
pub mod user_handler;
mod group_handler;

pub fn configure(cfg: &mut web::ServiceConfig) {
    common_handler::configure(cfg);
    auth_handler::configure(cfg);
    user_handler::configure(cfg);
    swagger::configure(cfg);
}
