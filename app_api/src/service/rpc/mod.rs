use crate::service::rpc::arb_client_service_impl::ArbClientServiceImpl;

pub mod arb_client_service_impl;

pub async fn init() {
    ArbClientServiceImpl::start().await;
}
