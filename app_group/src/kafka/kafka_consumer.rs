use anyhow::{anyhow, Result};
use bytes::Bytes;
use dashmap::DashMap;
use log::{debug, warn};
use once_cell::sync::OnceCell;
use prost::Message;
use rdkafka::message::{Message as KafkaMessageTrait, OwnedMessage};
use serde::Deserialize;
use std::sync::Arc;

use biz_service::manager::user_manager_core::{UserManager, UserManagerOpt};
use biz_service::protocol::common::ByteMessageType;
use common::config::KafkaConfig;
use common::util::date_util::now;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
use biz_service::biz_service::kafka_group_service::{GROUP_NODE_MSG_TOPIC};
use biz_service::protocol::msg::group::{ChangeGroupMsg, ChangeMemberRoleMsg, CreateGroupMsg, DestroyGroupMsg, ExitGroupMsg, GroupNodeMsgType, HandleInviteMsg, HandleJoinRequestMsg, InviteMembersMsg, MemberOnlineMsg, MuteMemberMsg, RemoveMembersMsg, RequestJoinGroupMsg, TransferOwnershipMsg, UpdateMemberProfileMsg};
use crate::manager::shard_manager::{ShardManager, ShardManagerMqOpt};

type MessageId = u64;

/// Kafka 消息分发结构体
#[derive(Debug, Deserialize)]
pub struct DispatchMessage {
    pub to: String,                    // 接收者用户 ID
    pub payload: String,               // 消息体（JSON/Proto 序列化后的字符串）
    pub message_id: Option<MessageId>, // 用于客户端确认的唯一标识
}

/// Kafka 消息元数据（用于 ACK 追踪）
pub struct PendingMeta {
    pub topic: String,
    pub partition: i32,
    pub offset: i64,
}

/// 全局未确认消息映射（msg_id -> 元信息）
static PENDING_ACKS: OnceCell<Arc<DashMap<MessageId, PendingMeta>>> = OnceCell::new();

pub fn get_pending_acks() -> Arc<DashMap<MessageId, PendingMeta>> {
    PENDING_ACKS.get_or_init(|| Arc::new(DashMap::new())).clone()
}

/// 全局 Kafka 消费者单例
static CONSUMER: OnceCell<Arc<StreamConsumer>> = OnceCell::new();

/// 获取 Kafka 消费者实例
pub fn get_consumer() -> Option<Arc<StreamConsumer>> {
    CONSUMER.get().cloned()
}

/// 启动 Kafka 消费循环
pub async fn start_consumer(kafka_cfg: &KafkaConfig) -> Result<()> {
    let consumer: StreamConsumer = ClientConfig::new()
        .set("group.id", "im-dispatch-group")
        .set("bootstrap.servers", kafka_cfg.brokers.clone())
        .set("enable.auto.commit", "false") // 手动提交 offset
        .create()?;

    consumer.subscribe(&[GROUP_NODE_MSG_TOPIC])?;
    log::info!("✅ Kafka 消费者已启动，订阅主题 {}", "group-node-msg");

    let arc_consumer = Arc::new(consumer);
    if CONSUMER.set(arc_consumer.clone()).is_err() {
        log::warn!("⚠️ Kafka CONSUMER 已初始化，跳过重复设置");
    }
    Ok(())
}
pub async fn handle_kafka_message(msg: &OwnedMessage) -> Result<()> {
    let payload = msg
        .payload()
        .ok_or_else(|| anyhow!("Kafka 消息为空"))?;

    if payload.is_empty() {
        return Err(anyhow!("Kafka 消息体为空"));
    }

    let msg_type = GroupNodeMsgType::try_from(payload[0] as i32)
        .map_err(|_| anyhow!("未知消息类型: {}", payload[0]))?;

    let body = &payload[1..];
    let shard_manager = ShardManager::get();

    match msg_type {
        // 创建群组
        GroupNodeMsgType::CreateGroupMsgType => {
            let msg = CreateGroupMsg::decode(body)?;
            shard_manager.create_group(&msg.group_id).await?;
        }
        // 销毁群组
        GroupNodeMsgType::DestroyGroupMsgType => {
            let msg = DestroyGroupMsg::decode(body)?;
            shard_manager.destroy_group(&msg).await?;
        }
        // 修改群组
        GroupNodeMsgType::ChangeGroupMsgType => {
            let msg = ChangeGroupMsg::decode(body)?; // 推荐改名后的结构
            shard_manager.change_group(&msg).await?;
        }
        // 处理群组成员相关消息
        GroupNodeMsgType::RequestJoinGroupMsgType => {
            let msg = RequestJoinGroupMsg::decode(body)?;
            shard_manager.request_join_group(&msg).await?;
        }
        // 处理入群申请
        GroupNodeMsgType::HandleJoinRequestMsgType => {
            let msg = HandleJoinRequestMsg::decode(body)?;
            shard_manager.handle_join_request(&msg).await?;
        }
        // 邀请群组成员
        GroupNodeMsgType::InviteMembersMsgType => {
            let msg = InviteMembersMsg::decode(body)?;
            shard_manager.invite_members(&msg).await?;
        }
        // 处理群组成员邀请
        GroupNodeMsgType::HandleInviteMsgType => {
            let msg = HandleInviteMsg::decode(body)?;
            shard_manager.handle_invite(&msg).await?;
        }
        // 移除群组成员
        GroupNodeMsgType::RemoveMembersMsgType => {
            let msg = RemoveMembersMsg::decode(body)?;
            shard_manager.remove_members(&msg).await?;
        }
        // 退出群组
        GroupNodeMsgType::ExitGroupMsgType => {
            let msg = ExitGroupMsg::decode(body)?;
            shard_manager.exit_group(&msg).await?;
        }
        // 修改群组成员角色
        GroupNodeMsgType::ChangeMemberRoleMsgType => {
            let msg = ChangeMemberRoleMsg::decode(body)?;
            shard_manager.change_member_role(&msg).await?;
        }
        // 禁言群组成员
        GroupNodeMsgType::MuteMemberMsgType => {
            let msg = MuteMemberMsg::decode(body)?;
            shard_manager.mute_member(&msg).await?; // ❗补 await
        }
        // 更新群组成员资料
        GroupNodeMsgType::UpdateMemberProfileMsgType => {
            let msg = UpdateMemberProfileMsg::decode(body)?;
            shard_manager.update_member_profile(&msg).await?;
        }
        // 群组转让
        GroupNodeMsgType::TransferOwnershipMsgType => {
            let msg = TransferOwnershipMsg::decode(body)?;
            shard_manager.transfer_owner_ship(&msg).await?;
        }
        GroupNodeMsgType::MemberOnlineMsgType => {
            let msg = MemberOnlineMsg::decode(body)?;
            shard_manager.member_online(&msg).await?;
        }
        GroupNodeMsgType::MemberOfflineMsgType => {
            let msg = MemberOnlineMsg::decode(body)?;
            shard_manager.member_offline(&msg).await?;
        }
        GroupNodeMsgType::UnknownMsgType | _ => {
            return Err(anyhow!("不支持的 GroupNodeMsgType: {}", payload[0]));
        }
    }

    Ok(())
}

