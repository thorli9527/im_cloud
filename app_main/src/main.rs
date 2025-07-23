use actix_web::middleware::Logger;
use actix_web::{App, HttpServer, web};
use app_main::handlers;
use biz_service::biz_service::kafka_socket_service::KafkaService;

use biz_service::rpc_client::client_util::ArbServerClient;
use common::config::AppConfig;
use log::warn;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    AppConfig::init(&"main-config.toml".to_string()).await;
    // 读取配置文件
    let app_cfg = AppConfig::get();
    let address_and_port = format!("{}:{}", &app_cfg.get_server().host, &app_cfg.get_server().port);
    warn!("Starting server on {}", address_and_port);
    KafkaService::init(&app_cfg.get_kafka()).await;
    ArbServerClient::init().await.expect("Failed to initialize gRPC clients");
    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            // 配置 控制器
            .configure(|cfg| {
                handlers::configure(cfg);
            })
    })
    .keep_alive(actix_web::http::KeepAlive::Timeout(std::time::Duration::from_secs(600))) // 允许 10 分钟超时
    .bind(address_and_port)?
    .run()
    .await
}
