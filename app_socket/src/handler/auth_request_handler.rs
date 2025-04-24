use crate::manager::socket_manager::{get_socket_manager, ConnectionId};
use crate::pb::message_util::build_auth_ack;
use crate::pb::protocol::{AuthRequest, Envelope};
use biz_service::biz_service::client_service::ClientService;
use biz_service::manager::user_manager::{DeviceType, RedisUserManager};
use common::errors::AppError;
use std::collections::HashSet;

pub async fn auth_request(id: &ConnectionId, data: Envelope, auth:AuthRequest) ->Result<(),AppError>{
    let user_profile_service= ClientService::get();
    let status = user_profile_service.verify_token(&auth.token).await?;
    let socket_manager = get_socket_manager();
    if status{
        //绑定socket用户信息
        let user = user_profile_service.find_client_by_token(&auth.token).await?;
        let mut conn_info = socket_manager.connections.get_mut(id).unwrap();
        conn_info.meta.user_id=Option::Some(user.user_id.clone());
        conn_info.meta.client_id=Option::Some(auth.client_id.clone());
        conn_info.meta.device_type=Option::Some(auth.device_type().clone());
        socket_manager.user_index.entry(user.user_id.clone()).or_insert_with(HashSet::new).insert(id.clone());

        //socket auth 请求回复
        let msg=build_auth_ack(data.envelope_id,auth.message_id,true,"".to_string());
        socket_manager.send_to_connection(id,msg).map_err(|_|AppError::SocketError("socket".to_string()))?;

        //redis设置用户在线
        RedisUserManager::get().online(&user.user_id,DeviceType::from(auth.device_type)).await?;
    }
    else{
        let msg=build_auth_ack(data.envelope_id,auth.message_id,false,"token.error".to_string());
        socket_manager.send_to_connection(id,msg).unwrap();
    }
    Ok(())
}