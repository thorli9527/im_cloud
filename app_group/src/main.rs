use crate::manager::shard_job::ArbManagerJob;
use actix_web::middleware::Logger;
use actix_web::{App, HttpServer};
use common::config::AppConfig;
use log::warn;

mod kafka;
mod manager;
mod service;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    AppConfig::init(&"group-config.toml".to_string()).await;
    // 读取配置文件
    let app_cfg = AppConfig::get();

    let address_and_port = format!("{}:{}", &app_cfg.get_server().host, &app_cfg.get_server().port);
    warn!("Starting server on {}", address_and_port);
    biz_service::init_service();
    manager::init().await;

    // 2. 构建 ShardManager 实例
    let mut job = ArbManagerJob::new();
    // 启动任务
    job.start().await;

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            // 配置 控制器
            .configure(|cfg| {
                // handlers::configure(cfg);
            })
    })
    .keep_alive(actix_web::http::KeepAlive::Timeout(std::time::Duration::from_secs(600))) // 允许 10 分钟超时
    .bind(address_and_port)?
    .run()
    .await
}
