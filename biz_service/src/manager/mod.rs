use crate::manager::group_manager_core::GroupManager;
use crate::manager::user_manager_core::UserManager;
use deadpool_redis::Pool;

pub mod common;
pub mod group_manager_core;
pub mod group_manager_impl;
pub mod user_manager_core;
pub mod user_manager_opt;

pub fn init() {
    UserManager::new();
    GroupManager::init();
}
