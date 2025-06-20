use crate::entitys::group_entity::GroupInfo;
use crate::manager::common::UserId;
use crate::manager::user_redis_manager::{UserManager, UserManagerOpt};
use dashmap::{DashMap, DashSet};
use once_cell::sync::OnceCell;
use std::hash::Hash;
use std::sync::Arc;

const SHARD_COUNT: usize = 16;
// === 分片群组结构 ===
#[derive(Debug, Clone)]
pub struct LocalGroupManager {
    group_info_map: Arc<DashMap<String, GroupInfo>>,
    group_members_shards_map: Arc<Vec<DashMap<String, DashSet<String>>>>,
    user_to_groups_shards: Arc<Vec<DashMap<String, DashSet<String>>>>,
}

pub trait LocalGroupManagerOpt: Send + Sync {
    /// 初始化群组
    fn init_group(&self, group_info: GroupInfo);
    ///获取群组信息
    /// # group_id: 群组ID
    fn get_group_info(&self, group_id: &str) -> Option<GroupInfo>;
    ///移除群组
    /// # group_id: 群组ID
    fn remove_group(&self, group_id: &str);
    /// 添加用户到群组
    /// # group_id: 群组ID
    /// # user_id: 用户ID
    fn add_user(&self, group_id: &str, user_id: &str);
    /// 移除用户从群组
    /// # group_id: 群组ID
    /// # user_id: 用户ID
    fn remove_user(&self, group_id: &str, user_id: &str);
    /// 获取群组用户列表
    /// # 返回用户ID列表
    /// # group_id: 群组ID
    fn get_users(&self, group_id: &str) -> Vec<UserId>;
    ///分页获群组用户列表
    /// # 返回用户ID列表
    /// # group_id: 群组ID
    /// # page: 页码，从0开始
    /// # page_size: 每页大小
    fn get_users_page(&self, group_id: &str, page: usize, page_size: usize) -> Vec<UserId>;
    /// 获取在线用户列表
    /// # group_id: 群组ID
    async fn get_online_users(&self, group_id: &str) -> Vec<UserId>;
    /// 获取离线用户列表
    /// # group_id: 群组ID
    async fn get_offline_users(&self, group_id: &str) -> Vec<UserId>;

    async fn get_user_groups(&self, user_id: &str) -> Vec<String>;

    /// 获取用户所在的群组，分页返回
    async fn get_user_groups_page(&self, user_id: &str, page: usize, page_size: usize) -> Vec<String> ;

    /// 判断用户是否在群组中
    async fn is_user_in_group(&self, group_id: &str, user_id: &UserId) -> bool;
}
impl LocalGroupManager {
    pub fn new() -> Self {
        let group_members_shards_map = Arc::new((0..SHARD_COUNT).map(|_| DashMap::new()).collect());
        let user_to_groups_shards = Arc::new((0..SHARD_COUNT).map(|_| DashMap::new()).collect());
        let group_info_map = Arc::new(DashMap::new());

        let result = Self {
            group_info_map,
            group_members_shards_map,
            user_to_groups_shards,
        };
        result.init(result.clone());
        result
    }

    /// 清理所有成员为空的群组（仅本地缓存）
    pub fn clean_empty_groups(&self) -> usize {
        let mut cleaned = 0;

        for shard in self.group_members_shards_map.iter() {
            let empty_groups: Vec<String> = shard
                .iter()
                .filter(|entry| entry.value().is_empty())
                .map(|entry| entry.key().clone())
                .collect();

            for group_id in empty_groups {
                shard.remove(&group_id);
                self.group_info_map.remove(&group_id);
                cleaned += 1;
                println!("[LocalGroupManager] 🧹 清理空群组: {}", group_id);
            }
        }

        println!("[LocalGroupManager] ✅ 本地清理完成，总计: {}", cleaned);
        cleaned
    }
    pub fn get_group_members_shard(&self, group_id: &str) -> &DashMap<String, DashSet<String>> {
        let hash = fxhash::hash64(group_id.as_bytes());
        let idx = (hash as usize) % SHARD_COUNT;
        &self.group_members_shards_map[idx]
    }
    pub fn get_user_to_groups_shards(&self, user_id: &str) -> &DashMap<String, DashSet<String>> {
        let hash = fxhash::hash64(user_id.as_bytes());
        let idx = (hash as usize) % SHARD_COUNT;
        &self.user_to_groups_shards[idx]
    }

    // 注册为全局单例
    fn init(&self, instance: LocalGroupManager) {
        INSTANCE.set(Arc::new(instance)).expect("INSTANCE already initialized");
    }

    // 获取全局实例
    pub fn get() -> Arc<Self> {
        INSTANCE.get().expect("INSTANCE is not initialized").clone()
    }

 
}
impl LocalGroupManagerOpt for LocalGroupManager {
    fn init_group(&self, group_info: GroupInfo) {
        let group_id = group_info.id.clone();
        self.group_info_map.insert(group_id, group_info);
    }

    fn get_group_info(&self, group_id: &str) -> Option<GroupInfo> {
        self.group_info_map.get(group_id).map(|v| v.clone())
    }

    fn remove_group(&self, group_id: &str) {
        self.group_info_map.remove(group_id);
    }

    fn add_user(&self, group_id: &str, user_id: &str) {
        let shard = self.get_group_members_shard(group_id);
        shard
            .entry(group_id.to_string())
            .or_insert_with(DashSet::new)
            .insert(user_id.to_string());

        let user_shard = self.get_user_to_groups_shards(user_id);
        user_shard
            .entry(user_id.to_string())
            .or_insert_with(DashSet::new)
            .insert(group_id.to_string());
    }

    fn remove_user(&self, group_id: &str, user_id: &str) {
        let shard = self.get_group_members_shard(group_id);
        if let Some(set) = shard.get(group_id) {
            set.remove(user_id);
        }

        let user_shard = self.get_user_to_groups_shards(user_id);
        if let Some(set) = user_shard.get(user_id) {
            set.remove(group_id);
        }
    }

    fn get_users(&self, group_id: &str) -> Vec<UserId> {
        let shard = self.get_group_members_shard(group_id);
        shard
            .get(group_id)
            .map(|set| set.iter().map(|v| v.to_string()).collect())
            .unwrap_or_default()
    }

    fn get_users_page(&self, group_id: &str, page: usize, page_size: usize) -> Vec<UserId> {
        let all_users = self.get_users(group_id);
        let start = page * page_size;
        let end = start + page_size;
        all_users.into_iter().skip(start).take(page_size).collect()
    }

    async fn get_online_users(&self, group_id: &str) -> Vec<UserId> {
        let group_info = self.get_group_info(group_id);
        if group_info.is_none() {
            return vec![];
        }

        let agent_id = group_info.unwrap().agent_id;
        let user_mgr = UserManager::get();

        let users = self.get_users(group_id);
        let mut result = Vec::with_capacity(users.len());
        for uid in &users {
            if user_mgr.is_online(&agent_id, uid).await.unwrap_or(false) {
                result.push(uid.clone());
            }
        }
        result
    }

    async fn get_offline_users(&self, group_id: &str) -> Vec<UserId> {
        let group_info = self.get_group_info(group_id);
        if group_info.is_none() {
            return vec![];
        }

        let agent_id = group_info.unwrap().agent_id;
        let user_mgr = UserManager::get();

        let users = self.get_users(group_id);
        let mut result = Vec::with_capacity(users.len());
        for uid in &users {
            if !user_mgr.is_online(&agent_id, uid).await.unwrap_or(true) {
                result.push(uid.clone());
            }
        }
        result
    }

    async fn get_user_groups(&self, user_id: &str) -> Vec<String> {
        let shard = self.get_user_to_groups_shards(user_id);
        shard
            .get(user_id)
            .map(|set| set.iter().map(|v| v.to_string()).collect())
            .unwrap_or_default()
    }

    /// 获取用户所在的群组，分页返回
    async fn get_user_groups_page(&self, user_id: &str, page: usize, page_size: usize) -> Vec<String> {
        let all = self.get_user_groups(user_id).await;
        let start = page * page_size;
        all.into_iter().skip(start).take(page_size).collect()
    }

    async fn is_user_in_group(&self, group_id: &str, user_id: &UserId) -> bool {
        // 1. 优先检查本地缓存
        let shard = self.get_group_members_shard(group_id);
        if let Some(members) = shard.get(group_id) {
            if members.contains(user_id) {
                return true;
            }
        }
        return false; 
    }
}

// 单例静态变量
static INSTANCE: OnceCell<Arc<LocalGroupManager>> = OnceCell::new();
