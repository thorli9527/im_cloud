pub mod shard_job;
mod shard_job_impl;
pub mod shard_manager;
mod shard_manager_mq_impl;
mod shard_manager_opt_impl;

pub async fn init() {
    shard_manager::ShardManager::init().await;
}
