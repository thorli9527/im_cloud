mod arb_client;
mod online_message_service;

pub async fn init_service() {
    online_message_service::OnLineMessageService::init().await;
    arb_client::ArbClient::init().await;
}
