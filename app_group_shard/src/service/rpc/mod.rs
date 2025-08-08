use biz_core::kafka_util::node_util::NodeUtil;

pub mod arb_client_service_impl;
mod arb_server_client;
pub mod shard_rpc_service_impl;

pub async fn init_service() {
    //初始化节点缓存信息
    NodeUtil::init().await;
}
