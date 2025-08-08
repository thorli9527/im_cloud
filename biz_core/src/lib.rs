pub mod consts;
pub mod service;
pub mod entitys;
pub mod kafka_util;
pub mod manager;
pub mod protocol;

pub async fn init_service() {
    service::init_service().await;
}
