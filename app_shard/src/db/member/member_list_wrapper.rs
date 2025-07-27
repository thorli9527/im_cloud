use crate::db::member::member_shard::SIMPLE_LIST_THRESHOLD;
use crate::db::member::sharded_member_list::ShardedMemberList;
use crate::db::member::simple_member_list::SimpleMemberList;
use arc_swap::ArcSwap;
use biz_service::protocol::arb::rpc_arb_models::MemberRef;
use biz_service::protocol::common::GroupRoleType;
use std::sync::Arc;

#[derive(Debug)]
enum MemberListImpl {
    Simple(Arc<SimpleMemberList>),
    Sharded(Arc<ShardedMemberList>),
}

#[derive(Debug)]
pub struct MemberListWrapper {
    inner: ArcSwap<MemberListImpl>,
}

impl Clone for MemberListWrapper {
    fn clone(&self) -> Self {
        Self {
            inner: ArcSwap::from(self.inner.load_full()),
        }
    }
}

impl MemberListWrapper {
    pub fn new_simple() -> Self {
        Self {
            inner: ArcSwap::from_pointee(MemberListImpl::Simple(Arc::new(SimpleMemberList::default()))),
        }
    }

    pub fn add(&self, item: MemberRef) {
        match self.inner.load().as_ref() {
            MemberListImpl::Simple(inner) => inner.add(item),
            MemberListImpl::Sharded(inner) => inner.add(item),
        }
    }

    pub fn add_many(&self, items: Vec<MemberRef>) {
        match self.inner.load().as_ref() {
            MemberListImpl::Simple(inner) => inner.add_many(items),
            MemberListImpl::Sharded(inner) => inner.add_many(items),
        }
    }

    pub fn remove(&self, id: &str) -> bool {
        match self.inner.load().as_ref() {
            MemberListImpl::Simple(inner) => inner.remove(id),
            MemberListImpl::Sharded(inner) => inner.remove(id),
        }
    }

    pub fn get_page(&self, page: usize, page_size: usize) -> Vec<MemberRef> {
        match self.inner.load().as_ref() {
            MemberListImpl::Simple(inner) => inner.get_page(page, page_size),
            MemberListImpl::Sharded(inner) => inner.get_page(page, page_size),
        }
    }

    pub fn len(&self) -> usize {
        match self.inner.load().as_ref() {
            MemberListImpl::Simple(inner) => inner.len(),
            MemberListImpl::Sharded(inner) => inner.total_len(),
        }
    }

    pub fn clear(&self) {
        match self.inner.load().as_ref() {
            MemberListImpl::Simple(inner) => inner.clear(),
            MemberListImpl::Sharded(inner) => inner.clear(),
        }
    }

    pub fn should_upgrade(&self) -> bool {
        matches!(self.inner.load().as_ref(), MemberListImpl::Simple(inner) if inner.len() > SIMPLE_LIST_THRESHOLD)
    }

    pub fn upgrade(&self, per_group_shard: usize) {
        let current = self.inner.load();
        if let MemberListImpl::Simple(inner) = current.as_ref() {
            let mut sharded = ShardedMemberList::new(per_group_shard);
            sharded.add_many(inner.get_all());
            self.inner.store(Arc::new(MemberListImpl::Sharded(Arc::new(sharded))));
        }
    }

    pub fn set_online(&self, id: &str, online: bool) {
        match self.inner.load().as_ref() {
            MemberListImpl::Simple(inner) => {
                if online {
                    inner.set_online(id)
                } else {
                    inner.set_offline(id)
                }
            }
            MemberListImpl::Sharded(inner) => {
                if online {
                    inner.set_online(id)
                } else {
                    inner.set_offline(id)
                }
            }
        }
    }

    pub fn get_online_ids(&self) -> Vec<String> {
        match self.inner.load().as_ref() {
            MemberListImpl::Simple(inner) => inner.get_online_all(),
            MemberListImpl::Sharded(inner) => inner.get_online_all(),
        }
    }

    pub fn get_online_count(&self) -> usize {
        match self.inner.load().as_ref() {
            MemberListImpl::Simple(inner) => inner.online_count(),
            MemberListImpl::Sharded(inner) => inner.online_count(),
        }
    }

    pub fn get_online_page(&self, page: usize, page_size: usize) -> Vec<String> {
        match self.inner.load().as_ref() {
            MemberListImpl::Simple(inner) => inner.get_online_page(page, page_size),
            MemberListImpl::Sharded(inner) => inner.get_online_page(page, page_size),
        }
    }
    pub fn get_all(&self) -> Vec<MemberRef> {
        match self.inner.load().as_ref() {
            MemberListImpl::Simple(inner) => inner.get_all(),
            MemberListImpl::Sharded(inner) => inner.get_all(),
        }
    }
    pub fn change_role(&self, user_id: &str, role: GroupRoleType) {
        match self.inner.load().as_ref() {
            MemberListImpl::Simple(inner) => inner.set_role(user_id, role),
            MemberListImpl::Sharded(inner) => inner.set_role(user_id, role),
        }
    }
}
