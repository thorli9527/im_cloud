use actix_web::{post, web, Responder};
use log::error;
use mongodb::bson::uuid;
use common::errors::AppError;
use tonic::Request;
use utoipa::ToSchema;
use biz_service::biz_service::group_member_service::GroupMemberService;
use biz_service::biz_service::group_service::GroupService;
use biz_service::protocol::common::{GroupEntity, GroupType, JoinPermission};
use biz_service::protocol::msg::auth::OnlineStatusMsg;
use biz_service::protocol::msg::group_models::{CreateGroupMsg, MemberOfflineMsg, MemberOnlineMsg};
use common::errors::AppError::Json;
use common::repository_util::{OrderType, Repository};
use crate::protocol::client_util::ArbServerClient;
use crate::protocol::rpc_group_models::ChangeGroupMsg;
use crate::result::{result, result_data, result_error};

/// 包装器类型（解决 Orphan Rule 限制）
pub struct CreateGroupMsgWrapper(pub CreateGroupMsg);

impl From<CreateGroupMsgWrapper> for GroupEntity {
    fn from(wrapper: CreateGroupMsgWrapper) -> Self {
        let msg = wrapper.0;
        let now = chrono::Utc::now().timestamp_millis() as u64;

        GroupEntity {
            id:"".to_string(),
            name: msg.name,
            avatar: msg.avatar,
            description: "".to_string(),
            notice: "".to_string(),
            join_permission: JoinPermission::Anyone as i32,
            owner_id: msg.creator_id,
            group_type: GroupType::NormalGroup as i32,
            allow_search: true,
            enable: true,
            create_time: now,
            update_time: now,
        }
    }
}
#[utoipa::path(
    post,
    path = "/group/add",
    request_body = CreateGroupMsg,
    responses(
        (status = 200, description = "群组创建成功")
    )
)]
#[post("/group/add")]
pub async fn group_add(req: web::Json<CreateGroupMsg>) -> Result<impl Responder, AppError> {
    let group_service = GroupService::get();
    let msg=CreateGroupMsgWrapper(req.clone());
    let group_entity=msg.into();
    let rs= group_service.create_group(&group_entity, &req.uids).await;
    return match rs {
        Ok(_) => {
            //创建 mq 消息
            Ok(result())
        },
        Err(e) => {
            log::error!("创建群组失败: {:?}", e);
            Ok(result_error(e.to_string()))
        }
    }
}



#[utoipa::path(
    post,
    path = "/group/update",
    request_body = ChangeGroupMsg,
    responses((status = 200, description = "群组更新成功"))
)]
#[post("/group/update")]
pub async fn group_update(req: web::Json<ChangeGroupMsg>) -> Result<impl Responder, AppError> {

    let group_service = GroupService::get();
    let group=group_service.find_by_group_id(&req.id).await?;

    Ok(result())
}

#[utoipa::path(
    post,
    path = "/group/online",
    request_body = MemberOnlineMsg,
    responses((status = 200, description = "群成员上线成功"))
)]
#[post("/group/online")]
pub async fn group_online(req: web::Json<MemberOnlineMsg>) -> Result<impl Responder, AppError> {
    Ok(result())
}

#[utoipa::path(
        post,
        path = "/group/offline",
        request_body = MemberOfflineMsg,
        responses((status = 200, description = "用户下线成功"))
    )]
    #[post("/group/offline")]
    pub async fn group_offline(req: web::Json<MemberOfflineMsg>) -> Result<impl Responder, AppError> {
        Ok(result())
    }

    struct MemberQuery {
        group_id: String,
        page_size: i64,
        order_type: OrderType,
    }
    pub async fn group_get_members(req: web::Json<MemberQuery>) -> Result<impl Responder, AppError> {
        Ok(result())
    }

    struct DestroyGroupReq {
        group_id: String,
        operator_id: String,
    }
    pub async fn group_destroy(req: web::Json<DestroyGroupReq>) -> Result<impl Responder, AppError> {
        let group_service = GroupService::get();
        group_service.dismiss_group(&req.group_id, &req.operator_id).await?;
        Ok(result())
    }


    pub async fn group_info(group_id: web::Path< String>) -> Result<impl Responder, AppError> {
        Ok(result())
    }



