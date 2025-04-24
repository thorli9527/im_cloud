mod auth_contoller;

use crate::result::AppState;
use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig, state: web::Data<AppState>) {}
