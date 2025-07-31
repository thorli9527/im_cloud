pub mod job_cache_cleaner;
pub mod job_manager;

pub fn configure() {
    // job_manager::start_heartbeat_cleaner(SocketManager::get(), 60);
    job_cache_cleaner::start();
}
