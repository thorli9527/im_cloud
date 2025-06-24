use actix_web::web;

pub mod group_member_page;
pub mod group_member_remove;
pub mod group_member_join;
pub mod group_member_refresh;

pub fn configure(cfg: &mut web::ServiceConfig) {
    group_member_remove::configure(cfg);
    group_member_page::configure(cfg);
    group_member_join::configure(cfg);
    group_member_refresh::configure(cfg);
}
