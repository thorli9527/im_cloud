use crate::manager::shard_job::ArbManagerJob;
use crate::manager::shard_manager::ShardManager;
use crate::service::rpc::group_rpc_service_impl;
use crate::service::rpc::group_rpc_service_impl::GroupRpcServiceImpl;
use actix_web::middleware::Logger;
use actix_web::{App, HttpServer};
use common::config::AppConfig;
use common::errors::AppError;
use deadpool_redis::{Pool, PoolConfig};
use mongodb::options::ClientOptions;
use mongodb::{Client, Database};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::log::{warn, LevelFilter};

mod manager;
mod protocol;
mod service;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    AppConfig::init(&"group-config.toml".to_string()).await;
    // 读取配置文件
    let app_cfg = AppConfig::get();
    //初始化日志
    init_log(&app_cfg.clone()).expect("init log error");

    let address_and_port = format!(
        "{}:{}",
        &app_cfg.get_server().host,
        &app_cfg.get_server().port
    );
    warn!("Starting server on {}", address_and_port);
    biz_service::init_service();
    manager::init_manager();
    // 2. 构建 ShardManager 实例
    let mut job = ArbManagerJob::new();
    // 启动任务
    job.start().await;

    // 启动 grpc 服务
    let group_rpc_service = GroupRpcServiceImpl::new();
    group_rpc_service.start().await;

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            // 配置 控制器
            .configure(|cfg| {
                // handlers::configure(cfg);
            })
    })
    .keep_alive(actix_web::http::KeepAlive::Timeout(
        std::time::Duration::from_secs(600),
    )) // 允许 10 分钟超时
    .bind(address_and_port)?
    .run()
    .await
}

pub fn init_log(config: &AppConfig) -> Result<(), AppError> {
    let mut builder = env_logger::Builder::new();
    let log_level = &config.get_sys().log_leve;
    let mut filter = builder.filter(None, LevelFilter::from_str(log_level).unwrap());
    filter.init();
    Ok(())
}
