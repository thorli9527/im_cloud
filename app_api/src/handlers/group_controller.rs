use crate::result::{result, result_data};
use actix_web::{Responder, post, web};
use biz_service::biz_service::agent_service::{AgentService, AuthHeader};
use biz_service::biz_service::group_member_service::GroupMemberService;
use biz_service::biz_service::group_service::GroupService;
use biz_service::biz_service::mq_group_operation_log_service::GroupOperationLogService;
use biz_service::entitys::group_entity::GroupInfo;
use biz_service::entitys::group_member::{GroupMember, GroupRole};
use biz_service::entitys::mq_group_operation_log::{GroupOperationLog, GroupOperationType};
use biz_service::manager::user_manager::RedisUserManager;
use common::errors::AppError;
use common::errors::AppError::BizError;
use common::repository_util::Repository;
use common::util::date_util::now;
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::{ToSchema, path};


