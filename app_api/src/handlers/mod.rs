mod user_contoller;
mod group;
mod group_member;
mod swagger;
mod common_handler;
mod message_handler;
mod frend_handler;

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    swagger::configure(cfg);
    common_handler::configure(cfg);
    group::configure(cfg);
    group_member::configure(cfg,);
    user_contoller::configure(cfg,);
    message_handler::configure(cfg);
    frend_handler::configure(cfg);
}
