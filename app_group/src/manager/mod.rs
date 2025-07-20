pub mod shard_manager;
pub mod shard_job;
mod shard_job_impl;
mod shard_manager_mq_impl;
mod shard_manager_opt_impl;

pub fn init_manager() {
    shard_manager::ShardManager::init();
}