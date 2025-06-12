use crate::result::AppState;
use actix_web::web;

mod group_member_page;
mod group_mute_all;
mod mute_member_add;
mod list_muted_members;
mod cancel_mute_member;
pub fn configure(cfg: &mut web::ServiceConfig, state: &web::Data<AppState>) {
    cancel_mute_member::configure(cfg,state);
    group_member_page::configure(cfg,state);
    group_mute_all::configure(cfg,state);
    mute_member_add::configure(cfg,state);
    list_muted_members::configure(cfg,state);
}
