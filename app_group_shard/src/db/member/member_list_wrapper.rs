use crate::db::member::sharded_member_list::ShardedMemberList;
use crate::db::member::simple_member_list::SimpleMemberList;
use crate::error::member_list_error::MemberListError;
use arc_swap::ArcSwap;
use biz_service::protocol::common::GroupRoleType;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use biz_service::protocol::arb::arb_models::MemberRef;

/// 每 10_000 个成员一个 shard，超过阈值后按照 hysteresis 规则平滑升级 / 扩容 / 收缩 / 降级。
const ITEMS_PER_SHARD: usize = 10_000;
const HYSTERESIS_DELTA: usize = 1_000;

#[derive(Debug, Clone)]
enum MemberListImpl {
    Simple(Arc<SimpleMemberList>),
    Sharded(Arc<ShardedMemberList>),
}

#[derive(Debug)]
struct MemberListWrapperInner {
    inner: ArcSwap<MemberListImpl>,
    epoch: AtomicU64,
}

#[derive(Clone, Debug)]
pub struct MemberListWrapper {
    inner: Arc<MemberListWrapperInner>, // 共享底层，clone 极轻
}

impl MemberListWrapper {
    pub fn new_simple() -> Self {
        Self {
            inner: Arc::new(MemberListWrapperInner {
                inner: ArcSwap::from_pointee(MemberListImpl::Simple(Arc::new(SimpleMemberList::default()))),
                epoch: AtomicU64::new(0),
            }),
        }
    }

    pub fn new_sharded(initial_shard: usize) -> Self {
        let shards = std::cmp::max(1, initial_shard);
        Self {
            inner: Arc::new(MemberListWrapperInner {
                inner: ArcSwap::from_pointee(MemberListImpl::Sharded(Arc::new(ShardedMemberList::new(shards)))),
                epoch: AtomicU64::new(0),
            }),
        }
    }

    fn current_epoch(&self) -> u64 {
        self.inner.epoch.load(Ordering::Acquire)
    }

    /// 读路径辅助
    fn with<R>(&self, on_simple: impl Fn(&SimpleMemberList) -> R, on_sharded: impl Fn(&ShardedMemberList) -> R) -> R {
        match self.inner.inner.load().as_ref() {
            MemberListImpl::Simple(inner) => on_simple(inner.as_ref()),
            MemberListImpl::Sharded(inner) => on_sharded(inner.as_ref()),
        }
    }

    /// 通用 mutation helper（不返回值），含 resize & epoch 检测
    fn mutate_with_retry<F, G>(&self, f_simple: F, f_sharded: G) -> Result<(), MemberListError>
    where
        F: Fn(&SimpleMemberList),
        G: Fn(&ShardedMemberList),
    {
        let before = self.current_epoch();
        match self.inner.inner.load().as_ref() {
            MemberListImpl::Simple(inner) => f_simple(inner.as_ref()),
            MemberListImpl::Sharded(inner) => f_sharded(inner.as_ref()),
        }
        self.try_resize_if_needed();
        if before != self.current_epoch() {
            return Err(MemberListError::Retry);
        }
        Ok(())
    }

    /// mutation helper for bool-returning ops like remove
    fn mutate_bool_with_retry<F, G>(&self, f_simple: F, f_sharded: G) -> Result<bool, MemberListError>
    where
        F: Fn(&SimpleMemberList) -> bool,
        G: Fn(&ShardedMemberList) -> bool,
    {
        let before = self.current_epoch();
        let result = match self.inner.inner.load().as_ref() {
            MemberListImpl::Simple(inner) => f_simple(inner.as_ref()),
            MemberListImpl::Sharded(inner) => f_sharded(inner.as_ref()),
        };
        self.try_resize_if_needed();
        if before != self.current_epoch() {
            return Err(MemberListError::Retry);
        }
        Ok(result)
    }

    /// 快照读取 + epoch，用于上层做 read-modify-retry
    pub fn snapshot_with_epoch(&self) -> (u64, Vec<MemberRef>) {
        let epoch = self.current_epoch();
        let all = self.get_all();
        (epoch, all)
    }

    pub fn len(&self) -> usize {
        self.with(|s| s.len(), |sh| sh.total_len())
    }

    pub fn add(&self, item: MemberRef) -> Result<(), MemberListError> {
        self.mutate_with_retry(|s| s.add(item.clone()), |sh| sh.add(item.clone()))
    }

    pub fn add_many(&self, items: Vec<MemberRef>) -> Result<(), MemberListError> {
        self.mutate_with_retry(|s| s.add_many(items.clone()), |sh| sh.add_many(items.clone()))
    }

    pub fn remove(&self, id: &str) -> Result<bool, MemberListError> {
        self.mutate_bool_with_retry(|s| s.remove(id), |sh| sh.remove(id))
    }

    pub fn get_page(&self, page: usize, page_size: usize) -> Vec<MemberRef> {
        self.with(|s| s.get_page(page, page_size), |sh| sh.get_page(page, page_size))
    }

    pub fn clear(&self) -> Result<(), MemberListError> {
        let before = self.current_epoch();
        match self.inner.inner.load().as_ref() {
            MemberListImpl::Simple(_) => {
                self.inner.inner.store(Arc::new(MemberListImpl::Simple(Arc::new(SimpleMemberList::default()))));
            }
            MemberListImpl::Sharded(inner) => {
                let mut new_sharded = (**inner).clone();
                new_sharded.clear();
                self.inner.inner.store(Arc::new(MemberListImpl::Sharded(Arc::new(new_sharded))));
            }
        }
        self.inner.epoch.fetch_add(1, Ordering::AcqRel);
        if before != self.current_epoch() {
            return Err(MemberListError::Retry);
        }
        Ok(())
    }

    pub fn set_online(&self, id: &str, online: bool) -> Result<(), MemberListError> {
        self.mutate_with_retry(
            |s| {
                if online { s.set_online(id) } else { s.set_offline(id) }
            },
            |sh| {
                if online { sh.set_online(id) } else { sh.set_offline(id) }
            },
        )
    }

    pub fn get_online_ids(&self) -> Vec<String> {
        self.with(|s| s.get_online_all(), |sh| sh.get_online_all())
    }

    pub fn get_online_count(&self) -> usize {
        self.with(|s| s.online_count(), |sh| sh.online_count())
    }

    pub fn get_online_page(&self, page: usize, page_size: usize) -> Vec<String> {
        self.with(|s| s.get_online_page(page, page_size), |sh| sh.get_online_page(page, page_size))
    }

    pub fn get_all(&self) -> Vec<MemberRef> {
        self.with(|s| s.get_all(), |sh| sh.get_all())
    }

    pub fn change_role(&self, user_id: &str, role: GroupRoleType) -> Result<(), MemberListError> {
        self.mutate_with_retry(|s| s.set_role(user_id, role), |sh| sh.set_role(user_id, role))
    }

    /// 根据当前大小和 hysteresis 规则决定是否要平滑调整（升级/扩容/收缩/降级），成功替换时 bump epoch
    fn try_resize_if_needed(&self) {
        let current_len = self.len();
        let loaded = self.inner.inner.load();

        match loaded.as_ref() {
            MemberListImpl::Simple(simple) => {
                if current_len <= ITEMS_PER_SHARD {
                    return;
                }
                if current_len > ITEMS_PER_SHARD + HYSTERESIS_DELTA {
                    let target_shards = std::cmp::max(1, current_len / ITEMS_PER_SHARD);
                    let mut new_sharded = ShardedMemberList::new(target_shards);
                    new_sharded.add_many(simple.get_all());
                    let new_impl = Arc::new(MemberListImpl::Sharded(Arc::new(new_sharded)));

                    // CAS-safe 替换
                    let prev = self.inner.inner.compare_and_swap(&loaded, new_impl);
                    if Arc::ptr_eq(&prev, &loaded) {
                        self.inner.epoch.fetch_add(1, Ordering::AcqRel);
                    }
                }
            }
            MemberListImpl::Sharded(sharded) => {
                let current_shards = sharded.shard_count;

                if current_len > current_shards * ITEMS_PER_SHARD + HYSTERESIS_DELTA {
                    // 扩一个 shard
                    let mut new_sharded = ShardedMemberList::new(current_shards + 1);
                    new_sharded.add_many(sharded.get_all());
                    let new_impl = Arc::new(MemberListImpl::Sharded(Arc::new(new_sharded)));
                    let prev = self.inner.inner.compare_and_swap(&loaded, new_impl);
                    if Arc::ptr_eq(&prev, &loaded) {
                        self.inner.epoch.fetch_add(1, Ordering::AcqRel);
                    }
                } else if current_shards > 1 && current_len < (current_shards - 1) * ITEMS_PER_SHARD.saturating_sub(HYSTERESIS_DELTA) {
                    // 收缩一个 shard
                    let mut new_sharded = ShardedMemberList::new(current_shards - 1);
                    new_sharded.add_many(sharded.get_all());
                    let new_impl = Arc::new(MemberListImpl::Sharded(Arc::new(new_sharded)));
                    let prev = self.inner.inner.compare_and_swap(&loaded, new_impl);
                    if Arc::ptr_eq(&prev, &loaded) {
                        self.inner.epoch.fetch_add(1, Ordering::AcqRel);
                    }
                } else if current_shards == 1 && current_len < ITEMS_PER_SHARD.saturating_sub(HYSTERESIS_DELTA) {
                    // 降级回 simple
                    let mut new_simple = SimpleMemberList::default();
                    new_simple.add_many(sharded.get_all());
                    let new_impl = Arc::new(MemberListImpl::Simple(Arc::new(new_simple)));
                    let prev = self.inner.inner.compare_and_swap(&loaded, new_impl);
                    if Arc::ptr_eq(&prev, &loaded) {
                        self.inner.epoch.fetch_add(1, Ordering::AcqRel);
                    }
                }
            }
        }
    }
}
