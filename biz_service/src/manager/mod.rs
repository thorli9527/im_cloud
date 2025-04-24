use deadpool_redis::Pool;

pub mod user_manager;

pub fn init(pool: Pool,node_id: usize, node_total: usize,use_local_cache: bool) {
    user_manager::RedisUserManager::new(pool,node_id,node_total,use_local_cache);
}
