mod online_message_service;
mod rpc;

pub async fn init_service() {
    online_message_service::OnLineMessageService::init().await;
}
