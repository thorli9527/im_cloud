use anyhow::anyhow;
use async_trait::async_trait;
use dashmap::{DashMap, DashSet};
use futures_util::StreamExt;
use mongodb::bson::doc;
use mongodb::bson::oid::ObjectId;
use mongodb::options::FindOptions;
use biz_service::biz_service::group_member_service::GroupMemberService;
use biz_service::biz_service::group_service::GroupService;
use biz_service::protocol::arb::rpc_arb_models::ShardState;
use biz_service::protocol::msg::group::{ChangeGroupMsg, ChangeMemberRoleMsg, DestroyGroupMsg, ExitGroupMsg, HandleInviteMsg, HandleJoinRequestMsg, InviteMembersMsg, MemberOnlineMsg, MuteMemberMsg, RemoveMembersMsg, RequestJoinGroupMsg, TransferOwnershipMsg, UpdateMemberProfileMsg};
use common::GroupId;
use crate::manager::shard_manager::{ShardManager, ShardManagerMqOpt, ShardManagerOpt};
#[async_trait]
impl ShardManagerMqOpt for ShardManager {
    async fn create_group(&self, group_id: &GroupId) -> anyhow::Result<()> {
        // Step 1: 计算分片索引 & 分片键
        let shard_index = self.hash_group_id(group_id) as i32;
        let shard_key = format!("shard_{}", shard_index);

        // Step 2: 检查当前分片状态
        let current = self.current.load();
        {
            let shard_info = current.shard_info.read().await;
            if shard_info.state != ShardState::Normal {
                tracing::log::warn!(
                    "❌ 无法添加群组 group_id={}，当前分片状态为 {:?}，非 Normal 状态",
                    group_id,
                    shard_info.state
                );
                return Ok(());
            }
        }

        // Step 3: 尝试添加到 group → shard 映射
        let group_map = current
            .group_shard_map
            .entry(shard_key.clone())
            .or_insert_with(DashMap::new);

        if !group_map.contains_key(group_id) {
            group_map.insert(group_id.clone(), ());
        } else {
            tracing::log::info!("⚠️ 群组已存在: group_id={} 分片={}", group_id, shard_index);
            return Ok(());
        }

        // Step 4: 拉取群成员信息（Redis 优先，数据库兜底）
        let group_members = match GroupMemberService::get().get_all_members_by_group_id(group_id).await {
            Ok(members) => members,
            Err(e) => {
                tracing::log::error!("❌ 获取群成员失败 group_id={} err={}", group_id, e);
                group_map.remove(group_id);
                return Err(e.into());
            }
        };

        // Step 5: 遍历成员，逐个加入分片成员槽
        for uid in &group_members {
            self.add_user_to_group(group_id, &uid.uid);
        }

        // Step 6: 初始化在线成员集合
        current
            .group_online_member_map
            .entry(shard_key)
            .or_insert_with(DashMap::new)
            .entry(group_id.clone())
            .or_insert_with(DashSet::new);

        tracing::log::info!(
            "✅ 成功添加群组: group_id={} → 分片 {} 成员数={}",
            group_id,
            shard_index,
            group_members.len()
        );
        Ok(())
    }

    async fn destroy_group(&self, msg: &DestroyGroupMsg) -> anyhow::Result<()> {
        let group_id = &msg.group_id;
        // 1. 计算分片索引
        let shard_index = self.hash_group_id(&group_id) as i32;
        let shard_key = format!("shard_{}", shard_index);

        // 2. 从 group_shard_map 中移除
        if let Some(group_map) = self.current.load().group_shard_map.get(&shard_key) {
            group_map.remove(group_id);
            if group_map.is_empty() {
                self.current.load().group_shard_map.remove(&shard_key);
            }
        }
        // 3. 从 group_member_map 中移除该群组的成员缓存
        if let Some(member_map) = self.current.load().group_member_map.get(&shard_key) {
            member_map.remove(group_id);
            if member_map.is_empty() {
                self.current.load().group_member_map.remove(&shard_key);
            }
        }
        //清除组成员在线
        if let Some(online_map) = self.current.load().group_online_member_map.get(&shard_key) {
            online_map.remove(group_id);
            if online_map.is_empty() {
                self.current
                    .load()
                    .group_online_member_map
                    .remove(&shard_key);
            }
        }
        // 4. 打日志记录
        tracing::log::info!(
            "❌ 群组删除成功: group_id={} 分片={}",
            group_id,
            shard_index
        );
        Ok(())
    }

    async fn change_group(&self, msg: &ChangeGroupMsg) -> anyhow::Result<()> {
        let online_members = self.get_online_users_for_group(&msg.group_id);
        todo!()
        
    }

    async fn request_join_group(&self, msg: &RequestJoinGroupMsg) -> anyhow::Result<()> {
        let online_members = self.get_online_users_for_group(&msg.group_id);

        todo!()
    }

    async fn handle_join_request(&self, msg: &HandleJoinRequestMsg) -> anyhow::Result<()> {
        todo!()
    }

    async fn invite_members(&self, msg: &InviteMembersMsg) -> anyhow::Result<()> {
        todo!()
    }

    async fn handle_invite(&self, msg: &HandleInviteMsg) -> anyhow::Result<()> {
        todo!()
    }

    async fn remove_members(&self, msg: &RemoveMembersMsg) -> anyhow::Result<()> {
        todo!()
    }

    async fn exit_group(&self, msg: &ExitGroupMsg) -> anyhow::Result<()> {
        todo!()
    }

    async fn change_member_role(&self, msg: &ChangeMemberRoleMsg) -> anyhow::Result<()> {
        todo!()
    }

    async fn mute_member(&self, msg: &MuteMemberMsg) -> anyhow::Result<()> {
        todo!()
    }

    async fn update_member_profile(&self, msg: &UpdateMemberProfileMsg) -> anyhow::Result<()> {
        todo!()
    }

    async fn transfer_owner_ship(&self, msg: &TransferOwnershipMsg) -> anyhow::Result<()> {
        todo!()
    }

    async fn member_online(&self, msg: &MemberOnlineMsg) -> anyhow::Result<()> {
        todo!()
    }

    async fn member_offline(&self, msg: &MemberOnlineMsg) -> anyhow::Result<()> {
        todo!()
    }
}
