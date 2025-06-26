use crate::manager::user_manager_core::UserManager;
use deadpool_redis::Pool;
use crate::manager::group_manager_core::GroupManager;

pub mod common;
pub mod local_group_manager;
pub mod user_manager_core;
pub mod user_manager_impl;
pub mod group_manager_core;
pub mod group_manager_impl;
mod local_group_manager_impl;

pub fn init(pool: Pool, use_local_cache: bool) {
    local_group_manager::LocalGroupManager::init();
    UserManager::new(pool.clone(),use_local_cache);
    GroupManager::init(pool.clone(), use_local_cache);
}
