use biz_core::protocol::common::GroupRoleType;
use dashmap::{DashMap, DashSet};
use std::sync::Arc;
use biz_core::protocol::arb::arb_models::MemberRef;

/// 简单的成员列表，适用于成员量较小时，内部用 dashmap 做并发。
#[derive(Debug, Default)]
pub struct SimpleMemberList {
    /// user_id (Arc<str>) -> MemberRef
    members: DashMap<Arc<str>, Arc<MemberRef>>,
    /// 在线 user_id 集合 (Arc<str>)
    online: DashSet<Arc<str>>,
}

impl SimpleMemberList {
    pub fn add(&self, member: MemberRef) {
        // 将 String 转为 Arc<str> 作为键
        let id_arc: Arc<str> = Arc::from(member.id.clone().into_boxed_str());
        self.members.insert(id_arc.clone(), Arc::new(member));
    }

    pub fn add_many(&self, items: Vec<MemberRef>) {
        for item in items {
            self.add(item);
        }
    }

    pub fn remove(&self, id: &str) -> bool {
        let key = Arc::from(id.to_string().into_boxed_str());
        let removed = self.members.remove(&key).is_some();
        self.online.remove(&key);
        removed
    }

    pub fn len(&self) -> usize {
        self.members.len()
    }

    /// 按 user_id 排序后的分页（保证结果稳定性）
    pub fn get_page(&self, page: usize, page_size: usize) -> Vec<MemberRef> {
        let mut all: Vec<MemberRef> = self.members.iter().map(|entry| entry.value().as_ref().clone()).collect();
        all.sort_by(|a, b| a.id.cmp(&b.id));
        let start = page.saturating_mul(page_size);
        if start >= all.len() {
            return Vec::new();
        }
        let end = (start + page_size).min(all.len());
        all[start..end].to_vec()
    }

    pub fn get_all(&self) -> Vec<MemberRef> {
        self.members.iter().map(|entry| entry.value().as_ref().clone()).collect()
    }

    pub fn set_online(&self, id: &str) {
        let key = Arc::from(id.to_string().into_boxed_str());
        self.online.insert(key);
    }

    pub fn set_offline(&self, id: &str) {
        let key = Arc::from(id.to_string().into_boxed_str());
        self.online.remove(&key);
    }

    pub fn get_online_all(&self) -> Vec<String> {
        let mut all: Vec<String> = self.online.iter().map(|entry| entry.key().to_string()).collect();
        all.sort();
        all
    }

    pub fn online_count(&self) -> usize {
        self.online.len()
    }

    pub fn get_online_page(&self, page: usize, page_size: usize) -> Vec<String> {
        let all = self.get_online_all();
        let start = page.saturating_mul(page_size);
        if start >= all.len() {
            return Vec::new();
        }
        let end = (start + page_size).min(all.len());
        all[start..end].to_vec()
    }

    pub fn set_role(&self, id: &str, role: GroupRoleType) {
        let key = Arc::from(id.to_string().into_boxed_str());
        if let Some(mut entry) = self.members.get_mut(&key) {
            let mut updated = (*entry).as_ref().clone();
            updated.role = role as i32;
            *entry = Arc::new(updated);
        }
    }

    pub fn clear(&self) {
        self.members.clear();
        self.online.clear();
    }
}
