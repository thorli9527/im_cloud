use actix_web::{post, web, Responder};
use log::error;
use mongodb::bson::uuid;
use common::errors::AppError;
use tonic::Request;
use utoipa::ToSchema;
use biz_service::biz_service::group_service::GroupService;
use biz_service::protocol::common::{GroupEntity, GroupType, JoinPermission};
use common::errors::AppError::Json;
use crate::protocol::client_util::ArbServerClient;
use crate::result::{result, result_data, result_error};

#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
#[derive(Clone, PartialEq,)]
pub struct CreateGroupDto {
    /// 可选自定义群ID
    pub id: String,
    /// 群名称
    pub name: String,
    /// 群头像URL
    pub avatar: String,
    /// 群简介
    pub description: String,
    /// 创建者用户ID
    pub creator_id: String,
    /// 初始成员用户ID列表
    pub member_ids: Vec<String>,
    /// 指定群主ID（可与创建者不同）
    pub owner_id:String,
    /// 是否需要审核加入
    pub need_approval: bool,
    /// 是否允许通过搜索加入
    pub allow_search: bool,
}
impl From<CreateGroupDto> for GroupEntity {
    fn from(req: CreateGroupDto) -> Self {
        let now = chrono::Utc::now().timestamp_millis() as u64;
        GroupEntity {
            id: "".to_string(),
            name: req.name,
            avatar: req.avatar,
            description: req.description,
            notice: "".to_string(), // 默认空公告
            join_permission: if req.need_approval {
                JoinPermission::NeedApproval as i32
            } else {
                JoinPermission::Anyone as i32
            },
            owner_id: req.owner_id,
            group_type: GroupType::NormalGroup as i32, // 默认普通群组，可根据需求调整
            allow_search: req.allow_search,
            enable: true,
            create_time: now,
            update_time: now,
        }
    }
}

#[utoipa::path(
    post,
    path = "/group/add",
    request_body = CreateGroupReq,
    responses(
        (status = 200, description = "群组创建成功")
    )
)]
#[post("/group/add")]
pub async fn group_add(req: web::Json<CreateGroupDto>) -> Result<impl Responder, AppError> {
    let group_service = GroupService::get();
    let group_entity = GroupEntity::from(req.clone());

    let rs= group_service.create_group(&group_entity, &req.member_ids).await;

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
    request_body = UpdateGroupInfoReq,
    responses((status = 200, description = "群组更新成功"))
)]
#[post("/group/update")]
pub async fn group_update(req: web::Json<UpdateGroupInfoReq>) -> Result<impl Responder, AppError> {
    let mut client = ArbServerClient::get_group_rpc_client(&req.id)?;
    let res = client.update_group_info(Request::new(req.into_inner())).await;
    Ok(result())
}

#[utoipa::path(
    post,
    path = "/group/online",
    request_body = OnlineReq,
    responses((status = 200, description = "群成员上线成功"))
)]
#[post("/group/online")]
pub async fn group_online(req: web::Json<OnlineReq>) -> Result<impl Responder, AppError> {
    let mut client = ArbServerClient::get_group_rpc_client(&req.group_id)?;
    let res = client.online(Request::new(req.into_inner())).await;
    match res {
        Ok(_) => {
            Ok(result())
        }
        Err(e) => {
            return Ok(result_error(format!("异常: {}", e)));
        }
    }
}

#[utoipa::path(
        post,
        path = "/group/offline",
        request_body = OfflineReq,
        responses((status = 200, description = "用户下线成功"))
    )]
    #[post("/group/offline")]
    pub async fn group_offline(req: web::Json<OfflineReq>) -> Result<impl Responder, AppError> {
        let mut client = ArbServerClient::get_group_rpc_client(&req.group_id)?;
        let res = client.offline(Request::new(req.into_inner())).await;
        Ok(result())
    }

    #[utoipa::path(
        post,
        path = "/group/online/users",
        request_body = GetOnlineMembersReq,
        responses((status = 200, description = "获取在线成员成功", body = GetOnlineMembersResp))
    )]
    #[post("/group/online/users")]
    pub async fn group_get_online(req: web::Json<GetOnlineMembersReq>) -> Result<impl Responder, AppError> {
        let mut client = ArbServerClient::get_group_rpc_client(&req.group_id)?;
        let res = client.get_online_members(Request::new(req.into_inner())).await;
        Ok(result())
    }

    #[utoipa::path(
        post,
        path = "/group/members",
        request_body = GetMembersReq,
        responses((status = 200, description = "获取群成员成功", body = GetMembersRep))
    )]
    #[post("/group/members")]
    pub async fn group_get_members(req: web::Json<GetMembersReq>) -> Result<impl Responder, AppError> {
        let mut client = ArbServerClient::get_group_rpc_client(&req.group_id)?;

        let res = client.get_members(Request::new(req.into_inner())).await;
        Ok(result())
    }

    #[utoipa::path(
        post,
        path = "/group/destroy",
        request_body = DestroyGroupReq,
        responses((status = 200, description = "解散群组成功"))
    )]
    #[post("/group/destroy")]
    pub async fn group_destroy(req: web::Json<DestroyGroupReq>) -> Result<impl Responder, AppError> {
        let mut client = ArbServerClient::get_group_rpc_client(&req.group_id)?;
        let res = client.destroy_group(Request::new(req.into_inner())).await;
        Ok(result())
    }

    #[utoipa::path(
        post,
        path = "/group/info",
        request_body = GetGroupInfoReq,
        responses((status = 200, description = "获取群组信息成功", body = GetGroupInfoRep))
    )]
    #[post("/group/info")]
    pub async fn group_info(req: web::Json<GetGroupInfoReq>) -> Result<impl Responder, AppError> {
        let mut client = ArbServerClient::get_group_rpc_client(&req.group_id)?;
        let res = client.get_group_info(Request::new(req.into_inner())).await;
        Ok(result())
    }

    pub fn configure(cfg: &mut web::ServiceConfig) {
        cfg.service(group_add)
            .service(group_update)
            .service(group_online)
            .service(group_offline)
            .service(group_get_online)
            .service(group_get_members)
            .service(group_destroy)
            .service(group_info);
    }


