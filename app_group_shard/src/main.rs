use crate::service::arb_manager::ArbManagerJob;
use crate::service::rpc::arb_group_service_impl::ArbGroupServiceImpl;
use crate::service::rpc::rpc_shard_server_impl::ShardRPCServiceImpl;
use actix_web::middleware::Logger;
use actix_web::web::service;
use actix_web::{App, HttpServer};
use biz_service::manager;
use biz_service::protocol::rpc::arb_group::arb_group_service_server::ArbGroupServiceServer;
use biz_service::protocol::rpc::shard_service::shard_rpc_service_server::ShardRpcServiceServer;
use common::config::AppConfig;
use log::warn;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;

mod db;
mod service;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    AppConfig::init(&"./app_group_shard/shard-config.toml".to_string()).await;
    // 读取配置文件
    let app_cfg = AppConfig::get();

    let address_and_port = format!("{}:{}", &app_cfg.get_server().host, &app_cfg.get_server().port);
    warn!("Starting server on {}", address_and_port);
    // 初始化 业务
    biz_service::manager::init();
    biz_service::init_service().await;

    service::init_service().await.expect("init service failed");
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
    let addr = SocketAddr::from_str(&app_cfg.get_shard().shard_address.unwrap()).expect("Invalid address");
    let abr_group_service = ArbGroupServiceServer::new(ArbGroupServiceImpl::new());
    let shard_group_service = ShardRpcServiceServer::new(ShardRPCServiceImpl::new());
    tonic::transport::Server::builder()
        .add_service(abr_group_service)
        .add_service(shard_group_service)
        .serve(addr)
        .await
        .expect("Failed to start server");
    log::warn!("ArbGroupServiceServer started");
}
