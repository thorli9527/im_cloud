use crate::manager::socket_manager::ConnectionId;
use anyhow::Result;
use biz_service::biz_service::client_service::ClientService;
use biz_service::manager::user_manager_core::{UserManager, UserManagerOpt};
use biz_service::protocol::auth::DeviceType;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

pub async fn handle_login(
    message_id: &u64,
    user_name: &str,
    password: &str,
    app_key: &str,
    device_type: &DeviceType,
) -> Result<String> {
    let user_manager = UserManager::get();
    let token = user_manager
        .login(message_id,user_name, password, app_key, device_type)
        .await?;
    return Ok(token);
}
