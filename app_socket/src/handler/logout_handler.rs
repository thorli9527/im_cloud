use crate::manager::socket_manager::ConnectionId;
use anyhow::Result;
use biz_service::manager::common::UserId;
use biz_service::manager::user_manager_core::{UserManager, UserManagerOpt};
use biz_service::protocol::auth::DeviceType;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};
pub async fn handle_logout(
    message_id: &u64,
    agent_id: &str,
    uid: &UserId,
    token: &str,
    device_type: &DeviceType,
) -> Result<()> {
    let user_manager = UserManager::get();
    user_manager
        .logout(message_id, agent_id, uid, device_type)
        .await?;
    user_manager.delete_token(token).await?;
    return Ok(());
}
