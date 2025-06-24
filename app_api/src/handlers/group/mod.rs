use actix_web::web;
use crate::handlers::group_member::group_member_join;

pub mod group_create;
pub mod group_quit;
pub mod group_dismiss;
pub mod group_refresh;
pub mod group_transfer;

pub fn configure(cfg: &mut web::ServiceConfig) {
    group_create::configure(cfg);
    group_member_join::configure(cfg);
    group_quit::configure(cfg);
    group_dismiss::configure(cfg);
    group_refresh::configure(cfg);
    group_transfer::configure(cfg);
}
