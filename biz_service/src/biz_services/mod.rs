use crate::biz_services::bucket_service::BucketService;
use crate::biz_services::path_service::PathService;
use crate::biz_services::user_service::UserService;
use actix_web::web;
use common::config::ServerRes;
use crate::biz_services::menu_service::MenuService;
use crate::biz_services::role_menu_service::RoleMenuService;
use crate::biz_services::role_service::RoleService;
use crate::biz_services::user_role_service::UserRoleService;

pub mod user_service;
pub mod bucket_service;
pub mod file_service;
pub mod path_service;
pub mod menu_service;
pub mod role_service;
pub mod role_menu_service;
pub mod user_role_service;

pub fn configure(cfg: &mut web::ServiceConfig, db_res:ServerRes) {

    let user_service=UserService::new(db_res.clone());
    cfg.app_data(web::Data::new(user_service));

    let bucket_service=BucketService::new(db_res.clone());
    cfg.app_data(web::Data::new(bucket_service));

    let bucket_service=BucketService::new(db_res.clone());
    cfg.app_data(web::Data::new(bucket_service));

    let path_service=PathService::new(db_res.clone());
    cfg.app_data(web::Data::new(path_service));

    let menu_service=MenuService::new(db_res.clone());
    cfg.app_data(web::Data::new(menu_service));

    let role_service=RoleService::new(db_res.clone());
    cfg.app_data(web::Data::new(role_service));

    let role_menu_service=RoleMenuService::new(db_res.clone());
    cfg.app_data(web::Data::new(role_menu_service));

    let user_role_service=UserRoleService::new(db_res.clone());
    cfg.app_data(web::Data::new(user_role_service));

}
