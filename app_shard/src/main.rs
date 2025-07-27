use crate::service::arb_group_service_impl::ArbGroupServiceImpl;
use crate::service::arb_manager::ArbManagerJob;
use actix_web::middleware::Logger;
use actix_web::{App, HttpServer};
use biz_service::manager;
use biz_service::protocol::arb::rpc_arb_group::arb_group_service_server::ArbGroupServiceServer;
use biz_service::protocol::arb::rpc_arb_socket::arb_socket_service_server::ArbSocketServiceServer;
use common::config::AppConfig;
use log::warn;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;

mod db;
mod service;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    AppConfig::init(&"./app_shard/group-config.toml".to_string()).await;
    // 读取配置文件
    let app_cfg = AppConfig::get();

    let address_and_port = format!("{}:{}", &app_cfg.get_server().host, &app_cfg.get_server().port);
    warn!("Starting server on {}", address_and_port);
    biz_service::init_service();
    start_grpc_server().await;
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
async fn start_grpc_server() {
    let app_cfg = AppConfig::get();
    let addr = SocketAddr::from_str(&app_cfg.get_shard().server_host.unwrap()).expect("Invalid address");
    let svc = ArbGroupServiceServer::new(ArbGroupServiceImpl::new());
    tonic::transport::Server::builder().add_service(svc).serve(addr).await.expect("Failed to start server");
    log::warn!("ArbGroupServiceServer started");
}
