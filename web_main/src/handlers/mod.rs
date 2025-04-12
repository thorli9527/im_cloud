use crate::result::AppState;
use actix_web::web;

mod swagger_handler;
mod common_handler;
mod user_handler;
mod path_handler;
mod bucket_handler;
mod file_handler;
mod auth_handler;

pub fn configure(cfg: &mut web::ServiceConfig, state: web::Data<AppState>) {
    common_handler::configure(cfg);
    swagger_handler::configure(cfg, state);
}
