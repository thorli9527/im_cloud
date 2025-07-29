use crate::socket::socket_manager::{ConnectionId, SocketManager};
use biz_service::manager::user_manager_auth::{UserManagerAuth, UserManagerAuthOpt};
use biz_service::protocol::common::ByteMessageType;
use biz_service::protocol::msg::auth::{AuthType, DeviceType, LoginRespMsg};
use biz_service::protocol::msg::status::AckMsg;
use log::warn;

pub async fn handle_login(
    conn_id: &ConnectionId,
    message_id: &u64,
    auth_type: &AuthType,
    auth_content: &str,
    password: &str,
    device_type: &DeviceType,
) {
    let socket_manager = SocketManager::get();
    let ack_msg = AckMsg {
        message_id: 0,
        ack_type: 0,
        success: false,
        error_code: 0,
    };
    let result = socket_manager.send_to_connection_proto(conn_id, &ByteMessageType::AckMsgType, &ack_msg);
    match result {
        Ok(_) => {
            log::info!("✅ 登录成功，发送登录成功消息给客户端");
        }
        Err(e) => {
            log::error!("❌ 登录失败，发送登录成功消息给客户端失败");
        }
    }

    let conn = socket_manager.get_by_id(conn_id);
    let user_manager_auth = UserManagerAuth::get();
    match user_manager_auth.login(message_id, auth_type, auth_content, password, device_type).await {
        Ok((token, client)) => {
            warn!("Login success: {}", auth_content);
            let msg = LoginRespMsg {
                message_id: *message_id,
                token,
                expires_at: 0,
                msg: "".to_string(),
                success: true,
                uid: client.uid,
                nickname: client.name,
                avatar: client.avatar,
            };
            socket_manager.send_to_connection_proto(conn_id, &ByteMessageType::LoginRespMsgType, &msg).unwrap();
        }
        Err(e) => {
            warn!("Login failed: {}", e);
            let msg = LoginRespMsg {
                message_id: *message_id,
                token: "".to_string(),
                expires_at: 0,
                msg: e.to_string(),
                uid: "".to_string(),
                nickname: "".to_string(),
                success: false,
                avatar: "".to_string(),
            };
            socket_manager.send_to_connection_proto(conn_id, &ByteMessageType::LoginRespMsgType, &msg).unwrap();
        }
    }
}
