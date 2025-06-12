mod user_contoller;
mod group;
mod group_member;
mod swagger;
mod common_handler;
use crate::result::AppState;
use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig, state: web::Data<AppState>) {
    swagger::configure(cfg);
    common_handler::configure(cfg);
    group::configure(cfg);
    group_member::configure(cfg, &state.clone());
    user_contoller::configure(cfg, &state.clone());
}
