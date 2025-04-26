mod user_contoller;
mod group_controller;
mod group_member_mute_controller;
mod group;
mod group_member;

use crate::result::AppState;
use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig, state: web::Data<AppState>) {}
