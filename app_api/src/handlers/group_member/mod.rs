use crate::result::AppState;
use actix_web::web;

mod group_member_page;
mod group_mute_all;
mod group_member_remove;
pub mod group_member_join;
pub mod group_member_refresh;

pub fn configure(cfg: &mut web::ServiceConfig, state: &web::Data<AppState>) {
    group_member_remove::configure(cfg, state);
    group_member_page::configure(cfg,state);
    group_mute_all::configure(cfg,state);
    group_mute_all::configure(cfg,state);
}
