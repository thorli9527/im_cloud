use crate::result::{result, result_error};
use actix_web::{post, web, Responder};
use biz_service::biz_service::group_service::GroupService;
use biz_service::protocol::common::{GroupEntity, GroupType, JoinPermission};
use common::errors::AppError;
use common::repository_util::{OrderType, Repository};
use mongodb::bson::oid::ObjectId;
use biz_service::protocol::msg::group::{ChangeGroupMsg, CreateGroupMsg};

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
    let mut group_entity: GroupEntity = msg.into();
    group_entity.id = ObjectId::new().to_hex();
    let rs= group_service.create_group(&group_entity, &req.uids).await;
    return match rs {
        Ok(_) => {
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
    let mut group = group_service.find_by_group_id(&req.id.to_string()).await?;
    let mut change_data = false;
    if !req.name.eq(&group.name) {
        change_data = true;
        group.name = req.name.clone();
    }
    if !req.description.eq(&group.description) {
        change_data = true;
        group.description = req.description.clone();
    }
    if !req.avatar.eq(&group.avatar) {
        change_data = true;
        group.avatar = req.avatar.clone();
    }
    if !req.notice.eq(&group.notice) {
        change_data = true;
        group.notice = req.notice.clone();
    }
    if change_data {
        group_service.dao.save(&group).await?;
    }
    Ok(result())
}



