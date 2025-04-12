use crate::biz_services::bucket_service::BucketService;
use crate::biz_services::path_service::PathService;
use crate::biz_services::user_service::UserService;
use actix_web::web;
use common::config::ServerRes;

pub mod user_service;
pub mod bucket_service;
pub mod file_service;
pub mod path_service;

pub fn configure(cfg: &mut web::ServiceConfig, db_res:ServerRes) {

    let user_service=UserService::new(db_res.clone());
    cfg.app_data(web::Data::new(user_service));

    let bucket_service=BucketService::new(db_res.clone());
    cfg.app_data(web::Data::new(bucket_service));

    let bucket_service=BucketService::new(db_res.clone());
    cfg.app_data(web::Data::new(bucket_service));

    let path_service=PathService::new(db_res.clone());
    cfg.app_data(web::Data::new(path_service));

}
