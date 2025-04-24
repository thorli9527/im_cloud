#![feature(unwrap_infallible)]

use deadpool_redis::Pool;
use mongodb::Database;

pub mod biz_const;
pub mod biz_service;
pub mod entitys;
pub mod manager;


pub fn init_service(db:Database){
    biz_service::init_service(db);
}
