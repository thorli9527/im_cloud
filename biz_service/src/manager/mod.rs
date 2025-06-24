use deadpool_redis::Pool;

pub mod user_redis_manager;
pub mod group_redis_manager;
pub mod common;
pub mod local_group_manager;

pub fn init(pool: Pool, use_local_cache: bool) {
    local_group_manager::LocalGroupManager::init();
    user_redis_manager::UserManager::new(pool.clone(),use_local_cache);
    group_redis_manager::GroupManager::init(pool.clone(), use_local_cache);
}
