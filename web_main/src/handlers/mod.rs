use crate::result::AppState;
use actix_web::web;

pub mod common_handler;
pub mod user_handler;
pub use user_handler::*;
pub mod path_handler;
pub use path_handler::*;
pub mod bucket_handler;
pub use bucket_handler::*;
pub mod file_handler;
pub mod auth_handler;
pub mod swagger;
pub use auth_handler::*;


pub fn configure(cfg: &mut web::ServiceConfig, state: web::Data<AppState>) {
    common_handler::configure(cfg);
    auth_handler::configure(cfg);
    user_handler::configure(cfg);
    swagger::configure(cfg);
}
