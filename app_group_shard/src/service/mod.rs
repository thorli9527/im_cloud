use crate::service::arb_manager::{ArbManagerJob, ManagerJobOpt};

pub mod arb_manager;
mod arb_manager_impl;
pub mod rpc;
pub mod shard_manager;
pub mod shard_manager_impl;
pub mod shard_manager_opt;
pub async fn init_service() -> anyhow::Result<()> {
    shard_manager::ShardManager::init().await;
    ArbManagerJob::init().await?;
    Ok(())
}
