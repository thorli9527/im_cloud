use biz_service::protocol::common::GroupRoleType;
use biz_service::protocol::rpc::arb_models::MemberRef;
use dashmap::{DashMap, DashSet};
use std::sync::Arc;

/// 简单的成员列表，适用于成员量较小时，内部用 dashmap 做并发。
#[derive(Debug, Default)]
pub struct SimpleMemberList {
    members: DashMap<String, Arc<MemberRef>>, // user_id -> MemberRef
    online: DashSet<String>,                  // online user_ids
}

impl SimpleMemberList {
    pub fn add(&self, member: MemberRef) {
        let id = member.id.clone();
        self.members.insert(id, Arc::new(member));
    }

    pub fn add_many(&self, items: Vec<MemberRef>) {
        for item in items {
            self.add(item);
        }
    }

    pub fn remove(&self, id: &str) -> bool {
        let removed = self.members.remove(id);
        self.online.remove(id);
        removed.is_some()
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
        self.online.insert(id.to_string());
    }

    pub fn set_offline(&self, id: &str) {
        self.online.remove(id);
    }

    pub fn get_online_all(&self) -> Vec<String> {
        let mut all: Vec<String> = self.online.iter().map(|entry| entry.key().clone()).collect();
        all.sort();
        all
    }

    pub fn online_count(&self) -> usize {
        self.online.len()
    }

    pub fn get_online_page(&self, page: usize, page_size: usize) -> Vec<String> {
        let mut all = self.get_online_all();
        let start = page.saturating_mul(page_size);
        if start >= all.len() {
            return Vec::new();
        }
        let end = (start + page_size).min(all.len());
        all[start..end].to_vec()
    }

    pub fn set_role(&self, id: &str, role: GroupRoleType) {
        if let Some(mut entry) = self.members.get_mut(id) {
            // clone, modify, replace to keep Arc semantics
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
