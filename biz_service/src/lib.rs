pub mod biz_const;
pub mod biz_service;
pub mod entitys;
pub mod manager;
pub mod protocol;
pub mod rpc_client;
pub async fn init_service() {
    biz_service::init_service().await;
}
