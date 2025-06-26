use crate::entitys::group_entity::GroupInfo;
use crate::entitys::group_member::{GroupMemberMeta, GroupRole};
use crate::manager::common::UserId;
use crate::manager::local_group_manager;
use crate::manager::local_group_manager::{LocalGroupManager, LocalGroupManagerOpt};
use crate::manager::user_manager_core::{UserManager, UserManagerOpt};
use async_trait::async_trait;
use dashmap::{DashMap, DashSet};
use once_cell::sync::OnceCell;
use std::sync::Arc;

impl LocalGroupManager {
    fn new() -> Self {
        let group_members_shards_map = Arc::new((0..local_group_manager::SHARD_COUNT).map(|_| DashMap::new()).collect());
        let group_members_meta_map = Arc::new((0..local_group_manager::SHARD_COUNT).map(|_| DashMap::new()).collect());
        let user_to_groups_shards = Arc::new((0..local_group_manager::SHARD_COUNT).map(|_| DashMap::new()).collect());
        let group_info_map = Arc::new(DashMap::new());

        Self { group_info_map, group_members_shards_map, group_members_meta_map, user_to_groups_shards }
    }

    /// 清理所有成员为空的群组（仅本地缓存）
    pub fn clean_empty_groups(&self) -> usize {
        let mut cleaned = 0;

        for shard in self.group_members_shards_map.iter() {
            let empty_groups: Vec<String> = shard.iter().filter(|entry| entry.value().is_empty()).map(|entry| entry.key().clone()).collect();

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
        let idx = (hash as usize) % crate::manager::local_group_manager::SHARD_COUNT;
        &self.group_members_shards_map[idx]
    }
    pub fn get_user_to_groups_shards(&self, user_id: &str) -> &DashMap<String, DashSet<String>> {
        let hash = fxhash::hash64(user_id.as_bytes());
        let idx = (hash as usize) % crate::manager::local_group_manager::SHARD_COUNT;
        &self.user_to_groups_shards[idx]
    }
    fn get_group_meta_shard(&self, group_id: &str) -> &DashMap<String, DashMap<String, GroupMemberMeta>> {
        let hash = fxhash::hash64(group_id.as_bytes());
        let idx = (hash as usize) % crate::manager::local_group_manager::SHARD_COUNT;
        &self.group_members_meta_map[idx]
    }

    // 注册为全局单例

    pub fn init() {
        let instance = Self::new();
        INSTANCE.set(Arc::new(instance)).expect("AgentService already initialized");
    }

    // 获取全局实例
    pub fn get() -> Arc<Self> {
        INSTANCE.get().expect("INSTANCE is not initialized").clone()
    }
}
// 单例静态变量
static INSTANCE: OnceCell<Arc<LocalGroupManager>> = OnceCell::new();
#[async_trait]
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

    fn add_user(&self, group_id: &str, user_id: &UserId, mute: Option<bool>, alias: &str, group_role: &GroupRole) {
        let group_id = group_id.to_string();
        let user_id = user_id.to_string();

        // 1. 加入群组成员列表
        self.get_group_members_shard(&group_id).entry(group_id.clone()).or_insert_with(DashSet::new).insert(user_id.clone());

        // 2. 加入用户到群组反向索引
        self.get_user_to_groups_shards(&user_id).entry(user_id.clone()).or_insert_with(DashSet::new).insert(group_id.clone());

        // 3. 写入群成员元信息（只在首次添加时插入）
        let meta_shard = self.get_group_meta_shard(&group_id);
        let meta_map = meta_shard.entry(group_id.clone()).or_insert_with(DashMap::new);

        meta_map.entry(user_id.clone()).or_insert_with(|| GroupMemberMeta {
            id: format!("{}_{}", group_id, user_id),
            group_id,
            uid: user_id,
            role: group_role.clone(),
            alias: Some(alias.to_string()),
            mute: mute.unwrap_or(false),
        });
    }

    fn refresh_user(&self, group_id: &str, user_id: &UserId, mute: Option<bool>, alias: &Option<String>, role: Option<GroupRole>) {
        let group_id = group_id.to_string();
        let user_id = user_id.to_string();

        // 1. 添加用户到群成员列表
        self.get_group_members_shard(&group_id).entry(group_id.clone()).or_insert_with(DashSet::new).insert(user_id.clone());

        // 2. 添加群组到用户映射
        self.get_user_to_groups_shards(&user_id).entry(user_id.clone()).or_insert_with(DashSet::new).insert(group_id.clone());

        // 3. 如果 alias、role、mute 都为空，直接跳过 meta 更新
        if alias.is_none() && role.is_none() && mute.is_none() {
            return;
        }

        // 4. 仅在 meta 存在时更新字段
        if let Some(group_map) = self.get_group_meta_shard(&group_id).get(&group_id) {
            if let Some(mut meta) = group_map.get_mut(&user_id) {
                if let Some(alias_str) = alias {
                    meta.alias = Some(alias_str.clone());
                }
                if let Some(new_role) = role {
                    meta.role = new_role;
                }
                if let Some(mute_flag) = mute {
                    meta.mute = mute_flag;
                }
            }
        }
    }

    fn remove_user(&self, group_id: &str, user_id: &UserId) {
        let group_id = group_id.to_string();
        let user_id = user_id.to_string();

        // 1. 移除群组 -> 用户 映射
        let shard = self.get_group_members_shard(&group_id);
        if let Some(set) = shard.get(&group_id) {
            set.remove(&user_id);
        }

        // 2. 移除用户 -> 群组 映射
        let user_shard = self.get_user_to_groups_shards(&user_id);
        if let Some(set) = user_shard.get(&user_id) {
            set.remove(&group_id);
        }

        // 3. 移除元信息缓存
        let meta_shard = self.get_group_meta_shard(&group_id);
        if let Some(meta_map) = meta_shard.get(&group_id) {
            meta_map.remove(&user_id);
        }
    }

    fn get_users(&self, group_id: &str) -> Vec<UserId> {
        let shard = self.get_group_members_shard(group_id);
        shard.get(group_id).map(|set| set.iter().map(|v| v.to_string()).collect()).unwrap_or_default()
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
        shard.get(user_id).map(|set| set.iter().map(|v| v.to_string()).collect()).unwrap_or_default()
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
