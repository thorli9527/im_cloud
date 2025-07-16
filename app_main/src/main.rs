use actix_web::middleware::Logger;
use actix_web::rt::Runtime;
use actix_web::test::init_service;
use actix_web::{cookie, web, App, HttpServer};
use app_main::handlers;
use app_main::handlers::swagger::openapi_json;
use app_main::result::AppState;
use biz_service::biz_service::kafka_service::KafkaService;
use biz_service::manager;
use common::config::AppConfig;
use common::errors::AppError;
use common::redis::redis_template::RedisTemplate;
use config::Config;
use deadpool_redis::redis::Client as redisClient;
use deadpool_redis::{
    redis::{cmd, FromRedisValue}, Connection, Manager, Pool, PoolConfig,
    Runtime as RedisRuntime,
};
use env_logger::Builder;
use log::{info, warn, LevelFilter};
use mongodb::options::ClientOptions;
use mongodb::{Client, Database};
use std::clone;
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    AppConfig::init(&"main-config.toml".to_string()).await;
    // 读取配置文件
    let app_cfg = AppConfig::get();
    //初始化日志
    init_log(&app_cfg.clone()).expect("init log error");
    let address_and_port = format!("{}:{}", &app_cfg.get_server().host, &app_cfg.get_server().port);
    warn!("Starting server on {}", address_and_port);
    KafkaService::init(&app_cfg.get_kafka()).await;
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

pub fn init_log(config: &AppConfig) -> Result<(), AppError> {
    let mut builder = env_logger::Builder::new();
    let log_level = &config.get_sys().log_leve;
    let mut filter = builder.filter(None, LevelFilter::from_str(log_level).unwrap());
    filter.init();
    Ok(())
}