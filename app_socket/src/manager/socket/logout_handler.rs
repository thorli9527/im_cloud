use anyhow::Result;
use biz_service::manager::user_manager_core::{UserManager, UserManagerOpt};
use biz_service::protocol::msg::auth::DeviceType;
use common::UserId;

pub async fn handle_logout(
    message_id: &u64,
    uid: &UserId,
    device_type: &DeviceType,
) -> Result<()> {
    let user_manager = UserManager::get();
    user_manager
        .logout(message_id,  uid, device_type)
        .await?;
    Ok(())
}
