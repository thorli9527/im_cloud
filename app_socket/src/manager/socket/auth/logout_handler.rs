use anyhow::Result;
use biz_service::manager::user_manager_auth::{UserManagerAuth, UserManagerAuthOpt};
use biz_service::protocol::msg::auth::DeviceType;
use common::UserId;

pub async fn handle_logout(message_id: &u64, uid: &UserId, device_type: &DeviceType) -> Result<()> {
    let user_manager_auth = UserManagerAuth::get();
    user_manager_auth.logout(message_id, uid, device_type).await?;
    Ok(())
}
