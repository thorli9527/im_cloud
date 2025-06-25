use actix_web::web;

pub mod common_handler;
pub mod user_handler;
mod agent_handler;
pub mod auth_handler;
pub mod swagger;

pub fn configure(cfg: &mut web::ServiceConfig) {
    common_handler::configure(cfg);
    auth_handler::configure(cfg);
    user_handler::configure(cfg);
    agent_handler::configure(cfg);
    swagger::configure(cfg);
}
