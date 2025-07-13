use anyhow::Result;
use biz_service::manager::user_manager_core::{UserManager, UserManagerOpt};
use biz_service::protocol::auth::DeviceType;

pub async fn handle_login(
    message_id: &u64,
    user_name: &str,
    password: &str,
    app_key: &str,
    device_type: &DeviceType,
) -> Result<String> {
    let user_manager = UserManager::get();
    let token = user_manager
        .login(message_id, user_name, password, device_type)
        .await?;
    Ok(token)
}
