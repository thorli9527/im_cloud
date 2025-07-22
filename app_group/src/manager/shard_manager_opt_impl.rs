use crate::manager::shard_job::ArbManagerJob;
use crate::manager::shard_manager::{GroupMembersPage, ShardManager, ShardManagerOpt, GROUP_SHARD_SIZE, MEMBER_SHARD_SIZE};
use async_trait::async_trait;
use biz_service::biz_service::group_member_service::GroupMemberService;
use biz_service::biz_service::group_service::GroupService;
use biz_service::protocol::common::GroupMemberEntity;
use common::config::AppConfig;
use common::util::common_utils::hash_index;
use common::{GroupId, UserId};
use dashmap::{DashMap, DashSet};
use futures_util::StreamExt;
use mongodb::bson::doc;
use mongodb::options::FindOptions;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use tonic::transport::Channel;
use twox_hash::XxHash64;
use biz_service::protocol::arb::rpc_arb_models::{NodeType, QueryNodeReq};

#[async_trait]
impl ShardManagerOpt for ShardManager {
    async fn load_from(&self) -> anyhow::Result<()> {
        let group_service = GroupService::get();
        let group_member_service = GroupMemberService::get();
        let collection = &group_service.dao.collection;
        let page_size = 100;
        let mut page = 0;
        let mut arb_manager_job = ArbManagerJob::new();
        arb_manager_job.init_arb_client().await?;
        let req = QueryNodeReq {
            node_type: NodeType::GroupNode as i32
        };
        let response = arb_manager_job.arb_client.unwrap().list_all_nodes(req).await?;

        let list = response.get_ref();

        let shard_addr = &AppConfig::get().shard.clone().unwrap().shard_address.unwrap();

        let shard_index = hash_index(shard_addr, list.nodes.len() as i32);

        loop {
            let skip = page * page_size;

            let find_options = FindOptions::builder()
                .projection(doc! { "_id": 1 }) // 只取 _id
                .limit(page_size as i64)
                .skip(skip as u64)
                .build();

            let mut cursor = collection.find(doc! {}).with_options(find_options).await?;

            let mut has_result = false;

            while let Some(doc) = cursor.next().await {
                has_result = true;

                // 获取 group_id
                let group_id = match doc {
                    Ok(d) => match d.get_object_id("_id") {
                        Ok(oid) => oid.to_hex(),
                        Err(_) => continue,
                    },
                    Err(_) => continue,
                };
                let group_index = hash_index(&group_id, list.nodes.len() as i32);
                if shard_index != group_index {
                    continue;
                }
                // 查询该群的所有成员
                let members: Vec<GroupMemberEntity> = group_member_service.get_all_members_by_group_id(&group_id).await?;;

                // 将每个成员添加到该群组分片中
                for member in members {
                    self.add_user_to_group(&group_id, &member.uid);
                }
            }
            if !has_result {
                break;
            }
            page += 1;
        }

        Ok(())
    }
    /// 计算群组分片索引（用于分配 group → shard）
    fn add_user_to_group(&self, group_id: &GroupId, uid: &UserId) {
        let shard_index = self.hash_group_id(group_id) as i32;
        let member_index = self.hash_group_member_id(group_id, uid);
        let shard_key = format!("shard_{}", shard_index);

        let current = self.current.load();

        let group_map = current
            .group_member_map
            .entry(shard_key.clone())
            .or_insert_with(DashMap::new);

        // 每个群组维护 N 个成员集合
        let member_shards = group_map.entry(group_id.clone()).or_insert_with(|| {
            let mut shards = Vec::with_capacity(MEMBER_SHARD_SIZE);
            for _ in 0..MEMBER_SHARD_SIZE {
                shards.push(DashSet::new());
            }
            shards
        });

        member_shards[member_index].insert(uid.clone());

        tracing::log::debug!(
            "👤 用户 {} 添加至群 {} 分片={} 成员槽={}",
            uid,
            group_id,
            shard_index,
            member_index
        );
    }
    
    /// 从指定群组中移除某个用户（自动计算分片）
    fn remove_user_from_group(&self, group_id: &GroupId, uid: &UserId) {
        let shard_index = self.hash_group_id(group_id) as i32;
        let member_index = self.hash_group_member_id(group_id, uid);
        let shard_key = format!("shard_{}", shard_index);

        let current = self.current.load();

        if let Some(group_map) = current.group_member_map.get(&shard_key) {
            if let Some(member_shards) = group_map.get(group_id) {
                if member_index >= member_shards.len() {
                    tracing::log::warn!(
                        "❌ 移除失败: 成员槽索引越界 group_id={} index={}",
                        group_id,
                        member_index
                    );
                    return;
                }

                // 从成员槽中移除用户
                if member_shards[member_index].remove(uid).is_some() {
                    tracing::log::debug!(
                        "👤 用户 {} 从群组 {} 成员槽 {} 移除（分片 {}）",
                        uid,
                        group_id,
                        member_index,
                        shard_index
                    );
                }

                // 如果该群组所有成员槽都为空，则清除该群组
                let group_empty = member_shards.iter().all(|slot| slot.is_empty());
                if group_empty {
                    group_map.remove(group_id);
                    tracing::log::debug!("⚠️ 群组 {} 无成员，已移除", group_id);
                }

                // 如果该分片已无任何群组，清除该 shard
                if group_map.is_empty() {
                    current.group_member_map.remove(&shard_key);
                    tracing::log::debug!("⚠️ 分片 {} 无群组缓存，已移除", shard_key);
                }
            }
        }
    }

    /// 获取某个群组的所有成员 ID 列表
    fn get_users_for_group(&self, group_id: &GroupId) -> Option<Vec<UserId>> {
        let shard_index = self.hash_group_id(group_id) as i32;
        let shard_key = format!("shard_{}", shard_index);

        let current = self.current.load();

        // 提前 clone 出用户集合
        if let Some(group_map) = current.group_member_map.get(&shard_key) {
            if let Some(member_shards) = group_map.get(group_id) {
                let users = member_shards
                    .iter()
                    .flat_map(|shard| shard.iter().map(|u| u.key().clone()))
                    .collect::<Vec<UserId>>();
                return Some(users);
            }
        }
        None
    }

    /// 获取群组成员分页列表
    fn get_group_members_page(
        &self,
        group_id: &GroupId,
        offset: usize,
        limit: usize,
    ) -> std::option::Option<Vec<UserId>> {
        let shard_index = self.hash_group_id(group_id) as i32;
        let shard_key = format!("shard_{}", shard_index);

        let current = self.current.load();

        if let Some(group_map) = current.group_member_map.get(&shard_key) {
            if let Some(member_shards) = group_map.get(group_id) {
                let all_users: Vec<UserId> = member_shards
                    .iter()
                    .flat_map(|shard| shard.iter().map(|u| u.key().clone()))
                    .skip(offset)
                    .take(limit)
                    .collect();
                return Some(all_users);
            }
        }

       return Option::None ;
    }
    fn get_group_member_total_count(&self, group_id: &GroupId) -> Option<usize> {
        let shard_index = self.hash_group_id(group_id) as i32;
        let shard_key = format!("shard_{}", shard_index);

        self.current
            .load()
            .group_member_map
            .get(&shard_key)
            .and_then(|group_map| {
                group_map.get(group_id).map(|member_shards| {
                    member_shards.iter().map(|shard| shard.len()).sum::<usize>()
                })
            })
    }
    fn mark_user_online(&self, group_id: &GroupId, uid: &UserId) {
        let shard_index = self.hash_group_id(&group_id) as i32;
        let shard_key = format!("shard_{}", shard_index);

        // 1. 插入在线用户 → 群组映射
        let guard = self.current.load();
        let group_map = guard
            .group_online_member_map
            .entry(shard_key.clone())
            .or_insert_with(DashMap::new);

        let user_set = group_map
            .entry(group_id.clone())
            .or_insert_with(DashSet::new);
        user_set.insert(uid.clone());
    }

    fn get_online_users_for_group(&self, group_id: &GroupId) -> Vec<UserId> {
        let shard_index = self.hash_group_id(&group_id) as i32;
        let shard_key = format!("shard_{}", shard_index);

        if let Some(group_map) = self.current.load().group_online_member_map.get(&shard_key) {
            if let Some(user_set) = group_map.get(group_id) {
                return user_set.iter().map(|u| u.key().clone()).collect();
            }
        }
        vec![]
    }
    fn mark_user_offline(&self, group_id: &GroupId, user_id: &UserId) {
        let shard_index = self.hash_group_id(&group_id) as i32;
        let shard_key = format!("shard_{}", shard_index);

        if let Some(group_map) = self.current.load().group_online_member_map.get(&shard_key) {
            if let Some(user_set) = group_map.get(group_id) {
                user_set.remove(user_id);

                if user_set.is_empty() {
                    group_map.remove(group_id);
                }

                if group_map.is_empty() {
                    self.current
                        .load()
                        .group_online_member_map
                        .remove(&shard_key);
                }
            }
        }
    }

    async fn get_admin_for_group(&self, group_id: &GroupId) -> anyhow::Result<Option<Vec<UserId>>> {
        let group_service = GroupService::get();
        let admin_member=group_service.query_admin_member_by_group_id(group_id).await?;
        Ok(Some(admin_member))
    }
}