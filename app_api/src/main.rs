use actix_web::middleware::Logger;
use app_api::handlers;
use common::config::AppConfig;

use actix_web::{App, HttpServer};
use tracing::log::warn;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // 读取配置文件
    AppConfig::init(&"api-config.toml".to_string()).await;
    let app_cfg = AppConfig::get();
    //初始化日志
    let address_and_port = format!("{}:{}", &app_cfg.get_server().host, &app_cfg.get_server().port);
    warn!("Starting server on {}", address_and_port);
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
