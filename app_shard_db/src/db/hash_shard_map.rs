use crate::db::member::member_list_wrapper::MemberListWrapper;
use crate::db::member::simple_member_list::SimpleMemberList;
use biz_service::protocol::arb::rpc_arb_models::MemberRef;
use biz_service::protocol::common::GroupRoleType;
use dashmap::DashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use twox_hash::XxHash64;

/// 单个分片结构，存储群组 ID → 成员列表
#[derive(Debug, Default)]
struct Shard {
    inner: DashMap<String, Arc<MemberListWrapper>>,
}

#[derive(Debug, Clone)]
pub struct HashShardMap {
    shards: Arc<Vec<Arc<Shard>>>,
    per_group_shard: usize,
}

impl HashShardMap {
    /// 创建 HashShardMap
    pub fn new(shard_count: usize, per_group_shard: usize) -> Self {
        let shards = (0..shard_count).map(|_| Arc::new(Shard::default())).collect();
        Self {
            shards: Arc::new(shards),
            per_group_shard,
        }
    }

    /// 获取指定 key 对应的分片索引
    fn get_shard_index(&self, key: &str) -> usize {
        let mut hasher = XxHash64::with_seed(0);
        key.hash(&mut hasher);
        (hasher.finish() as usize) % self.shards.len()
    }

    /// 获取 key 对应分片
    fn get_shard(&self, key: &str) -> Arc<Shard> {
        let idx = self.get_shard_index(key);
        self.shards[idx].clone()
    }

    /// 获取所有群组 ID
    pub fn all_keys(&self) -> Vec<String> {
        /// self.shards.read().unwrap().iter().flat_map(|shard| shard.inner.iter().map(|entry| entry.key().clone())).collect()
        self.shards.iter().flat_map(|shard| shard.inner.iter().map(|entry| entry.key().clone())).collect()
    }

    /// 群组成员计数
    pub fn get_item_count(&self, key: &str) -> usize {
        self.get_shard(key).inner.get(key).map(|list| list.len()).unwrap_or(0)
    }

    /// 插入单个成员
    pub fn insert(&self, key: String, member: MemberRef) {
        if key.is_empty() {
            return;
        }

        let shard = self.get_shard(&key);
        let mut entry = shard.inner.entry(key.clone()).or_insert_with(|| Arc::new(MemberListWrapper::Simple(Arc::new(SimpleMemberList::default()))));

        entry.add(member);

        if entry.should_upgrade() {
            let upgraded = entry.upgrade(self.per_group_shard);
            *entry = Arc::new(upgraded);
        }
    }

    /// 批量插入成员
    pub fn insert_many(&self, key: &str, members: &[MemberRef]) {
        if key.is_empty() {
            return;
        }

        let shard = self.get_shard(key);
        let mut entry =
            shard.inner.entry(key.to_string()).or_insert_with(|| Arc::new(MemberListWrapper::Simple(Arc::new(SimpleMemberList::default()))));

        entry.add_many(members.to_vec());

        if entry.should_upgrade() {
            let upgraded = entry.upgrade(self.per_group_shard);
            *entry = Arc::new(upgraded);
        }
    }

    /// 获取分页成员列表
    pub fn get_page(&self, key: &str, page: usize, page_size: usize) -> Option<Vec<MemberRef>> {
        self.get_shard(key).inner.get(key).map(|list| list.get_page(page, page_size))
    }

    /// 获取所有成员
    pub fn get_all_items(&self, key: &str) -> Option<Vec<MemberRef>> {
        self.get_shard(key).inner.get(key).map(|list| list.get_page(0, usize::MAX))
    }

    /// 修改成员角色（仅在成员存在时生效）
    pub fn change_member_role(&self, key: &str, id: &str, new_role: GroupRoleType) {
        let shard = self.get_shard(key);
        if let Some(entry) = shard.inner.get(key) {
            let mut items = entry.get_page(0, usize::MAX);
            let mut changed = false;
            for member in items.iter_mut() {
                if member.id == id {
                    member.role = new_role as i32;
                    changed = true;
                    break;
                }
            }
            if changed {
                entry.clear();
                entry.add_many(items);
            }
        }
    }

    /// 删除整个群组记录
    pub fn remove_key(&self, key: &str) -> bool {
        let shard = self.get_shard(key);
        shard.inner.remove(key).is_some()
    }

    /// 删除某个成员
    pub fn remove_item(&self, key: &str, member_id: &str) -> bool {
        self.get_shard(key).inner.get(key).map(|list| list.remove(member_id)).unwrap_or(false)
    }

    /// 删除多个成员
    pub fn remove_item_list(&self, key: &str, member_ids: &[String]) -> bool {
        self.get_shard(key).inner.get(key).map(|list| member_ids.iter().fold(false, |acc, id| acc | list.remove(id))).unwrap_or(false)
    }

    /// 扩容分片：重新计算并分配已有群组
    pub fn expand_shards(&mut self, new_shard_count: usize) {
        let old_shards = self.shards.clone();
        let mut new_shards = vec![Arc::new(Shard::default()); new_shard_count];

        for shard in old_shards.iter() {
            for entry in shard.inner.iter() {
                let key = entry.key();
                let mut hasher = XxHash64::with_seed(0);
                key.hash(&mut hasher);
                let idx = (hasher.finish() as usize) % new_shard_count;
                new_shards[idx].inner.insert(key.clone(), entry.value().clone());
            }
        }

        self.shards = Arc::new(new_shards);
    }

    pub fn set_online(&self, key: &str, id: &str, online: bool) {
        if key.is_empty() || id.is_empty() {
            return;
        }
        if let Some(entry) = self.get_shard(key).inner.get(key) {
            entry.set_online(id, online);
        }
    }
    pub fn get_online_members(&self, key: &str) -> Vec<MemberRef> {
        if key.is_empty() {
            return vec![];
        }
        if let Some(entry) = self.get_shard(key).inner.get(key) {
            return entry.get_online_members();
        }
        vec![]
    }
    pub fn get_online_count(&self, key: &str) -> usize {
        if key.is_empty() {
            return 0;
        }
        if let Some(entry) = self.get_shard(key).inner.get(key) {
            return entry.get_online_count();
        }
        0
    }
    pub fn get_online_page(&self, key: &str, page: usize, page_size: usize) -> Vec<MemberRef> {
        if key.is_empty() {
            return vec![];
        }
        if let Some(entry) = self.get_shard(key).inner.get(key) {
            return entry.get_online_page(page, page_size);
        }
        vec![]
    }
}
