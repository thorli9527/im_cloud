use actix_web::web;

mod register_handler;
mod register_handler_dto;
mod reset_password_handler;
mod reset_password_handler_dto;

pub fn configure(cfg: &mut web::ServiceConfig) {
    register_handler::configure(cfg);
    reset_password_handler::configure(cfg);
}
