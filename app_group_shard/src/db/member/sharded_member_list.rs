use std::hash::Hasher;
use std::sync::Arc;
use twox_hash::XxHash64;
use biz_core::protocol::arb::arb_models::MemberRef;
use crate::db::member::simple_member_list::SimpleMemberList;
use biz_core::protocol::common::GroupRoleType;

/// 支持分片的成员列表，内部每个 shard 是一个 SimpleMemberList。
#[derive(Debug, Clone)]
pub struct ShardedMemberList {
    pub shard_count: usize,
    pub shards: Vec<Arc<SimpleMemberList>>,
}

impl ShardedMemberList {
    pub fn new(shard_count: usize) -> Self {
        let shards = (0..std::cmp::max(1, shard_count)).map(|_| Arc::new(SimpleMemberList::default())).collect();
        Self {
            shard_count: std::cmp::max(1, shard_count),
            shards,
        }
    }

    /// 根据 user id 选 shard（一致的 xxhash64 with seed 0）
    fn shard(&self, id: &str) -> &SimpleMemberList {
        let mut hasher = XxHash64::with_seed(0);
        hasher.write(id.as_bytes());
        let idx = (hasher.finish() as usize) % self.shard_count;
        // unwrap safe because shards.len() == shard_count >= 1
        &*self.shards[idx]
    }

    /// 加一个成员（按 id hash 到某个 shard）
    pub fn add(&self, member: MemberRef) {
        self.shard(&member.id).add(member);
    }

    /// 批量添加，逐个分发到各自 shard
    pub fn add_many(&self, items: Vec<MemberRef>) {
        for item in items {
            self.add(item);
        }
    }

    /// 删除某个 member id（会在它所属 shard 上删除）
    pub fn remove(&self, id: &str) -> bool {
        self.shard(id).remove(id)
    }

    /// 总成员数（各 shard 之和）
    pub fn total_len(&self) -> usize {
        self.shards.iter().map(|s| s.len()).sum()
    }

    /// 取所有成员（合并所有 shard）；注意顺序不是强保证的
    pub fn get_all(&self) -> Vec<MemberRef> {
        self.shards.iter().flat_map(|s| s.get_all()).collect()
    }

    /// 分页：合并后统一分页
    pub fn get_page(&self, page: usize, page_size: usize) -> Vec<MemberRef> {
        let mut all = self.get_all();
        let start = page.saturating_mul(page_size);
        if start >= all.len() {
            return Vec::new();
        }
        let end = (start + page_size).min(all.len());
        all.drain(start..end).collect()
    }

    /// 在线 id 列表（合并所有 shard）
    pub fn get_online_all(&self) -> Vec<String> {
        self.shards.iter().flat_map(|s| s.get_online_all()).collect()
    }

    /// 在线人数
    pub fn online_count(&self) -> usize {
        self.shards.iter().map(|s| s.online_count()).sum()
    }

    /// 在线分页
    pub fn get_online_page(&self, page: usize, page_size: usize) -> Vec<String> {
        let mut all = self.get_online_all();
        let start = page.saturating_mul(page_size);
        if start >= all.len() {
            return Vec::new();
        }
        let end = (start + page_size).min(all.len());
        all.drain(start..end).collect()
    }

    /// 设置某个成员在线状态
    pub fn set_online(&self, id: &str) {
        self.shard(id).set_online(id);
    }

    pub fn set_offline(&self, id: &str) {
        self.shard(id).set_offline(id);
    }

    /// 改角色
    pub fn set_role(&self, id: &str, role: GroupRoleType) {
        self.shard(id).set_role(id, role);
    }

    /// 清空所有 shard（保留 shard_count 结构）
    pub fn clear(&self) {
        for shard in &self.shards {
            shard.clear();
        }
    }
}
