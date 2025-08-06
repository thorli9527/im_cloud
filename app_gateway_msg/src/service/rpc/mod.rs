use crate::service::rpc::arb_client_service_impl::ArbClientServiceImpl;

pub mod arb_client_service_impl;
pub mod arb_server_client;

pub async fn init() {
    ArbClientServiceImpl::init().await;
}
