use crate::db::member::sharded_member_list::ShardedMemberList;
use crate::db::member::simple_member_list::SimpleMemberList;
use biz_service::protocol::arb::rpc_arb_models::MemberRef;
use std::sync::Arc;
use crate::db::sharded_list::SIMPLE_LIST_THRESHOLD;

#[derive(Debug)]
pub enum MemberListWrapper {
    Simple(Arc<SimpleMemberList>),
    Sharded(Arc<ShardedMemberList>),
}
impl Clone for MemberListWrapper {
    fn clone(&self) -> Self {
        match self {
            MemberListWrapper::Simple(inner) => MemberListWrapper::Simple(inner.clone()),
            MemberListWrapper::Sharded(inner) => MemberListWrapper::Sharded(inner.clone()),
        }
    }
}

impl MemberListWrapper {
    pub fn add(&self, item: MemberRef) {
        match self {
            MemberListWrapper::Simple(inner) => inner.add(item),
            MemberListWrapper::Sharded(inner) => inner.add(item),
        }
    }

    pub fn add_many(&self, items: Vec<MemberRef>) {
        match self {
            MemberListWrapper::Simple(inner) => inner.add_many(items),
            MemberListWrapper::Sharded(inner) => inner.add_many(items),
        }
    }

    pub fn remove(&self, id: &str) -> bool {
        match self {
            MemberListWrapper::Simple(inner) => inner.remove(id),
            MemberListWrapper::Sharded(inner) => inner.remove(id),
        }
    }

    pub fn get_page(&self, page: usize, page_size: usize) -> Vec<MemberRef> {
        match self {
            MemberListWrapper::Simple(inner) => inner.get_page(page, page_size),
            MemberListWrapper::Sharded(inner) => inner.get_page(page, page_size),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            MemberListWrapper::Simple(inner) => inner.len(),
            MemberListWrapper::Sharded(inner) => inner.total_len(),
        }
    }

    pub fn clear(&self) {
        match self {
            MemberListWrapper::Simple(inner) => inner.clear(),
            MemberListWrapper::Sharded(inner) => inner.clear(),
        }
    }

    pub fn should_upgrade(&self) -> bool {
        matches!(self, MemberListWrapper::Simple(inner) if inner.len() > SIMPLE_LIST_THRESHOLD)
    }

    pub fn upgrade(self: &Arc<Self>, per_group_shard: usize) -> Self {
        match self.as_ref() {
            MemberListWrapper::Simple(inner) => {
                let sharded = ShardedMemberList::new(per_group_shard);
                sharded.add_many(inner.get_all());
                MemberListWrapper::Sharded(Arc::new(sharded))
            }
            MemberListWrapper::Sharded(_) => self.as_ref().clone(),
        }
    }

    pub fn maybe_upgrade(&mut self, per_group_shard: usize) {
        if let MemberListWrapper::Simple(inner) = self {
            if inner.len() > SIMPLE_LIST_THRESHOLD {
                let new_sharded = ShardedMemberList::new(per_group_shard);
                new_sharded.add_many(inner.get_all());
                *self = MemberListWrapper::Sharded(Arc::new(new_sharded));
            }
        }
    }
    pub fn set_online(&self, id: &str, online: bool) {
        match self {
            MemberListWrapper::Simple(inner) => inner.set_online(id, online),
            MemberListWrapper::Sharded(inner) => inner.set_online(id, online),
        }
    }
    pub fn get_online_members(&self) -> Vec<MemberRef> {
        match self {
            MemberListWrapper::Simple(inner) => inner.get_all().into_iter().filter(|m| inner.is_online(&m.id)).collect(),
            MemberListWrapper::Sharded(inner) => {
                let mut result = Vec::new();
                for shard in inner.shards.iter() {
                    result.extend(shard.get_all().into_iter().filter(|m| shard.is_online(&m.id)));
                }
                result
            }
        }
    }

    pub fn get_online_count(&self) -> usize {
        self.get_online_members().len()
    }

    pub fn get_online_page(&self, page: usize, page_size: usize) -> Vec<MemberRef> {
        self.get_online_members().into_iter().skip(page * page_size).take(page_size).collect()
    }
}
