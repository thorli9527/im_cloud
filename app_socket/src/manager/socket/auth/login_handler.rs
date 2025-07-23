use crate::manager::socket_manager::{ConnectionId, SocketManager};
use biz_service::manager::user_manager_auth::{UserManagerAuth, UserManagerAuthOpt};
use biz_service::protocol::common::ByteMessageType;
use biz_service::protocol::msg::auth::{DeviceType, LoginRespMsg};

pub async fn handle_login(
    conn_id: &ConnectionId,
    message_id: &u64,
    user_name: &str,
    password: &str,
    device_type: &DeviceType,
) {
    let socket_manager = SocketManager::get();
    let conn = socket_manager.get_by_id(conn_id);
    let user_manager_auth = UserManagerAuth::get();
    match user_manager_auth.login(message_id, user_name, password, device_type).await {
        Ok(token) => {
            let msg = LoginRespMsg { message_id: *message_id, token: token.clone(), expires_at: 0 };
            socket_manager
                .send_to_connection_proto(conn_id, &ByteMessageType::LoginRespMsgType, &msg)
                .unwrap();
            // 登录成功，处理 token
            println!("Login successful, token: {}", token);
        }
        Err(e) => {
            // 登录失败，处理错误
            eprintln!("Login failed: {}", e);
        }
    }
}
