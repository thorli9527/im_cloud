pub mod client_service;
pub mod country_service;
mod friend_event_service;
pub mod friend_service;
pub mod group_member_service;
pub mod group_service;
pub mod kafka_group_service;
pub mod kafka_socket_service;
pub mod mail_service;
pub mod mq_message_group_service;
pub mod mq_message_user_service;
pub mod role_service;
pub mod user_role_service;
pub mod user_service;

use crate::biz_service::friend_event_service::FriendEventService;
use common::db::Db;

pub async fn init_service() {
    let db = Db::get();
    client_service::ClientService::init(db.clone()).await;
    country_service::CountryService::init(db.clone()).await;
    group_member_service::GroupMemberService::init(db.clone()).await;
    group_service::GroupService::init(db.clone()).await;
    user_service::UserService::init(db.clone()).await;
    mq_message_group_service::GroupMessageService::init(db.clone()).await;
    mq_message_user_service::UserMessageService::init(db.clone()).await;
    friend_service::UserFriendService::init(db.clone()).await;
    FriendEventService::init(db.clone()).await;
}
