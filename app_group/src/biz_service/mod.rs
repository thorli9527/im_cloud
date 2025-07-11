use mongodb::Database;
use common::config::AppConfig;
use crate::biz_service::kafka_service::KafkaService;

pub mod rpc;
pub mod group_service;
pub mod group_member_service;
pub mod kafka_service;
pub fn init_service(db: Database) {
    group_service::GroupService::init(db.clone());
    group_member_service::GroupMemberService::init(db.clone());
    let app_config=AppConfig::get().clone();
    //新启线程
    tokio::spawn(async move {
        // 初始化 Kafka 服务
        KafkaService::init(&app_config.kafka.clone().unwrap()).await;
    });

}