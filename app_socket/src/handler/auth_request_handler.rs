use crate::manager::socket_manager::{ConnectionId, get_socket_manager};
use anyhow::{Context, Result};
use biz_service::biz_service::client_service::ClientService;
use biz_service::manager::user_manager_core::UserManager;
use std::collections::HashSet;
use biz_service::protocol::auth::AuthRequest;
use biz_service::protocol::envelope::Envelope;
pub async fn auth_request(id: &ConnectionId, data: Envelope, auth: AuthRequest) -> Result<()> {
    let user_profile_service = ClientService::get();
    let arc = UserManager::get();
    let user_manager = UserManager::get();
    let status = user_profile_service.verify_token(&auth.token).await?;
    let socket_manager = get_socket_manager();
    if status {
        //绑定socket用户信息
        let user = user_profile_service.find_client_by_token(&auth.token).await.context("find client by token error")?.unwrap();
        let mut conn_info = socket_manager.connections.get_mut(id).unwrap();
        conn_info.meta.user_id = Option::Some(user.uid.clone());
        conn_info.meta.client_id = Option::Some(auth.uid.clone());
        conn_info.meta.device_type = Option::Some(auth.device_type().clone());
        socket_manager.user_index.entry(user.uid.clone()).or_insert_with(HashSet::new).insert(id.clone());

        //socket auth 请求回复
        // let msg=build_auth_ack(data.envelope_id,auth.message_id,true,"".to_string());
        // socket_manager.send_to_connection(id,msg).map_err(|_|AppError::SocketError("socket".to_string()))?;

        //redis设置用户在线
        // let user_id:UserId= user.uid.clone();
        // UserManager::get().online(&user.agent_id, &user_id, DeviceType::from(auth.device_type as u8)).await?;
    } else {
        // let msg=build_auth_ack(data.envelope_id,auth.message_id,false,"token.error".to_string());
        // socket_manager.send_to_connection(id,msg).unwrap();
    }
    Ok(())
}
