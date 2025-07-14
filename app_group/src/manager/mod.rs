use mongodb::Database;

pub mod shard_manager;
pub mod shard_job;
mod shard_job_impl;
pub fn init_manager() {
    shard_manager::ShardManager::init();
}