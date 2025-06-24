pub mod agent_service;
pub mod cache_service;
pub mod client_service;
pub mod country_service;
pub mod user_service;
pub mod group_service;
pub mod group_member_service;
pub mod mq_group_application_service;
pub mod mq_group_operation_log_service;
pub mod mq_user_action_service;
pub mod mq_message_group_service;
pub mod mq_message_user_service;
mod kafka_service;

use mongodb::Database;
use common::config::KafkaConfig;

pub fn init_service(db: Database, kafka_config: KafkaConfig) {
    agent_service::AgentService::init(db.clone());
    client_service::ClientService::init(db.clone());
    country_service::CountryService::init(db.clone());
    group_member_service::GroupMemberService::init(db.clone());
    group_service::GroupService::init(db.clone());
    user_service::UserService::init(db.clone());
    mq_group_application_service::GroupApplicationService::init(db.clone());
    mq_group_operation_log_service::GroupOperationLogService::init(db.clone());
    mq_user_action_service::UserActionLogService::init(db.clone());
    mq_message_group_service::GroupMessageService::init(db.clone());
    mq_message_user_service::UserMessageService::init(db.clone());
    kafka_service::KafkaService::init(kafka_config);
}

