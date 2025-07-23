// use crate::manager::group_manager_core::GroupManager;
use crate::manager::user_manager::UserManager;

pub mod common;
pub mod group_manager_core;
pub mod group_manager_impl;
pub mod user_manager;
pub mod user_manager_auth;
pub mod user_manager_auth_impl;
pub mod user_manager_opt;

pub fn init() {
    UserManager::new();
    // GroupManager::init();
}
