#![feature(unwrap_infallible)]

use mongodb::Database;

pub mod biz_const;
pub mod biz_service;
pub mod entitys;
pub mod manager;
pub mod protocol;
pub fn init_service() {
    biz_service::init_service();
}
