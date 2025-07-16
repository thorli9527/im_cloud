pub mod client_service;
pub mod country_service;
mod friend_event_service;
pub mod friend_service;
pub mod group_member_service;
pub mod group_service;
pub mod kafka_service;
pub mod mq_message_group_service;
pub mod mq_message_user_service;
pub mod user_service;
pub mod permission_service;
pub mod role_service;
pub mod user_role_service;
pub mod menu_service;
pub mod role_permission_service;

use crate::biz_service::friend_event_service::FriendEventService;
use common::db::Db;

pub fn init_service() {
    let db = Db::get();
    client_service::ClientService::init(db.clone());
    country_service::CountryService::init(db.clone());
    group_member_service::GroupMemberService::init(db.clone());
    group_service::GroupService::init(db.clone());
    user_service::UserService::init(db.clone());
    mq_message_group_service::GroupMessageService::init(db.clone());
    mq_message_user_service::UserMessageService::init(db.clone());
    friend_service::UserFriendService::init(db.clone());
    FriendEventService::init(db.clone());
}
