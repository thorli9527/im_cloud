use actix_web::web;

mod group_create;
mod group_join;
mod group_quit;
mod group_dismiss;
mod group_refresh;
mod group_transfer;
mod group_admin_add;
mod group_admin_remove;

pub fn configure(cfg: &mut web::ServiceConfig) {
    group_create::configure(cfg);
    group_join::configure(cfg);
    group_quit::configure(cfg);
    group_dismiss::configure(cfg);
    group_refresh::configure(cfg);
    group_transfer::configure(cfg);
    group_admin_add::configure(cfg);
    group_admin_remove::configure(cfg);
}
