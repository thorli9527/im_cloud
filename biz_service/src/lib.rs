pub mod biz_const;
pub mod biz_service;
pub mod entitys;
pub mod kafka_util;
pub mod manager;
pub mod protocol;

pub async fn init_service() {
    biz_service::init_service().await;
}
