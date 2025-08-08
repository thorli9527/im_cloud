use crate::service::online_message_service::OnLineMessageService;

mod kafka_service;
mod online_message_service;
mod rpc;

pub async fn init_service() {
    OnLineMessageService::init().await;
    rpc::init().await;
    kafka_service::KafkaService::init().await.expect("instance kafka error");
}
