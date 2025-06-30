mod common_handler;
mod friend_handler;
mod group;
mod group_member;
mod message_handler;
mod swagger;
mod user_controller;
mod auth_handler;

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    swagger::configure(cfg);
    common_handler::configure(cfg);
    group::configure(cfg);
    group_member::configure(cfg);
    user_controller::configure(cfg);
    message_handler::configure(cfg);
    friend_handler::configure(cfg);
    auth_handler::configure(cfg);
}
