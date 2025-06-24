#![feature(unwrap_infallible)]

use mongodb::Database;
use common::config::KafkaConfig;

pub mod biz_const;
pub mod biz_service;
pub mod entitys;
pub mod manager;


pub fn init_service(db:Database,kafka_config: KafkaConfig){
    biz_service::init_service(db,kafka_config);
}
