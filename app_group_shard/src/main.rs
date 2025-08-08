use crate::service::arb_manager::ArbManagerJob;
use crate::service::rpc::arb_client_service_impl::ArbClientServiceImpl;
use crate::service::rpc::shard_rpc_service_impl::ShardRpcServiceImpl;
use actix_web::middleware::Logger;
use actix_web::web::service;
use actix_web::{App, HttpServer};
use biz_core::service::init_service;
use biz_core::manager;
use common::config::AppConfig;
use log::warn;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use biz_core::protocol::arb::arb_client::arb_client_service_server::ArbClientServiceServer;
use biz_core::protocol::arb::shard_service::shard_rpc_service_server::ShardRpcServiceServer;

mod db;
mod error;
mod service;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    AppConfig::init(&"./app_group_shard/shard-config.toml".to_string()).await;
    // 读取配置文件
    let app_cfg = AppConfig::get();

    let address_and_port = format!("{}:{}", &app_cfg.get_server().host, &app_cfg.get_server().port);
    warn!("Starting server on {}", address_and_port);
    // 初始化 业务
    manager::init();
    init_service().await;
    service::init_service().await.expect("init service failed");
    start_grpc(&app_cfg).await;
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

async fn start_grpc(app_cfg: &Arc<AppConfig>) {
    // 读取配置文件
    let addr = SocketAddr::from_str(&app_cfg.get_shard().client_addr.unwrap()).expect("Invalid address");
    let arb_client_service = ArbClientServiceImpl::new();
    let shard_rpc_service = ShardRpcServiceImpl::new();
    tonic::transport::Server::builder()
        .add_service(ShardRpcServiceServer::new(shard_rpc_service))
        .add_service(ArbClientServiceServer::new(arb_client_service))
        .serve(addr)
        .await
        .expect("Failed to start tonic gRPC server");
}
