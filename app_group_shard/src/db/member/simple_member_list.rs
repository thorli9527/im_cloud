use biz_service::protocol::common::GroupRoleType;
use dashmap::{DashMap, DashSet};
use std::sync::Arc;
use biz_service::protocol::rpc::arb_models::MemberRef;

#[derive(Debug, Default)]
pub struct SimpleMemberList {
    pub members: DashMap<String, Arc<MemberRef>>, // ID -> 成员对象
    online_map: DashSet<String>,                  // 仅存储在线成员
}

impl SimpleMemberList {
    pub fn new() -> Self {
        Self {
            members: DashMap::new(),
            online_map: DashSet::new(),
        }
    }

    pub fn add(&self, item: MemberRef) {
        if item.id.is_empty() {
            return;
        }
        self.members.insert(item.id.clone(), Arc::new(item));
    }

    pub fn add_many(&self, items: Vec<MemberRef>) {
        for item in items {
            if !item.id.is_empty() {
                self.members.insert(item.id.clone(), Arc::new(item));
                // 不自动上线
            }
        }
    }

    pub fn remove(&self, id: &str) -> bool {
        let removed = self.members.remove(id).is_some();
        self.online_map.remove(id);
        removed
    }

    pub fn get_all(&self) -> Vec<MemberRef> {
        self.members.iter().map(|entry| entry.value().as_ref().clone()).collect()
    }

    pub fn get_page(&self, page: usize, page_size: usize) -> Vec<MemberRef> {
        let mut all = self.get_all();
        all.sort_by(|a, b| a.id.cmp(&b.id));
        all.into_iter().skip(page * page_size).take(page_size).collect()
    }

    pub fn len(&self) -> usize {
        self.members.len()
    }

    pub fn clear(&self) {
        self.members.clear();
        self.online_map.clear();
    }

    pub fn set_online(&self, id: &str) {
        self.online_map.insert(id.to_string());
    }

    pub fn set_offline(&self, id: &str) {
        self.online_map.remove(id);
    }

    pub fn is_online(&self, id: &str) -> bool {
        self.online_map.contains(id)
    }

    pub fn online_count(&self) -> usize {
        self.online_map.len()
    }

    /// 返回在线成员 ID 分页
    pub fn get_online_page(&self, page: usize, page_size: usize) -> Vec<String> {
        let mut ids: Vec<String> = self.online_map.iter().map(|id| id.clone()).collect();
        ids.sort();
        ids.into_iter().skip(page * page_size).take(page_size).collect()
    }

    /// 返回所有在线成员 ID
    pub fn get_online_all(&self) -> Vec<String> {
        self.online_map.iter().map(|id| id.clone()).collect()
    }

    pub fn set_role(&self, id: &str, role: GroupRoleType) {
        if let Some(mut entry) = self.members.get_mut(id) {
            // 克隆原始数据（Arc<MemberRef>），创建可变副本
            let mut updated = (**entry).clone();
            updated.role = role as i32;
            // 替换为新的 Arc
            *entry = Arc::new(updated);
        }
    }
}
