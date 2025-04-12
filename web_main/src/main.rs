use actix_web::middleware::Logger;
use actix_web::rt::Runtime;
use actix_web::{cookie, web, App, HttpServer};
use biz_service::biz_services;
use biz_service::biz_services::configure;
use config::Config;
use deadpool_redis::redis::Client as redisClient;
use deadpool_redis::{redis::{cmd, FromRedisValue}, Connection, Manager, Pool, PoolConfig, Runtime as RedisRuntime};
use env_logger::Builder;
use log::{info, warn, LevelFilter};
use mongodb::options::ClientOptions;
use mongodb::{Client, Database};
use std::clone;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use common::config::{AppConfig, ServerRes};
use common::errors::AppError;
use common::redis::redis_template::RedisTemplate;
use web_main::handlers;
use web_main::result::AppState;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // 读取配置文件
    let app_state = AppState::new();
    //初始化日志
    init_log(&app_state.config);
    let address_and_port = format!("{}:{}", &app_state.config.server.host, &app_state.config.server.port);
    warn!("Starting server on {}", address_and_port);
    let redis_pool = build_redis_pool(&app_state.config);
    let redis_template=RedisTemplate::new(redis_pool);
    let res_service=ServerRes{redis_template,db:init_mongo_db(&app_state.config).await};
    let web_state=web::Data::new(app_state.clone());
    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            //配置 服务
            .configure(|cfg| {
                biz_services::configure(cfg,res_service.clone())
            })
            // 配置 控制器
            .configure(|cfg| {
                handlers::configure(cfg, web_state.clone());
            })
    })
    .keep_alive(actix_web::http::KeepAlive::Timeout(
        std::time::Duration::from_secs(600),
    )) // 允许 10 分钟超时
    .bind(address_and_port)?
    .run()
    .await
}


pub fn init_log(config:&AppConfig)->Result<(),AppError>{
    let mut builder = env_logger::Builder::new();
    let log_level =&config.sys.log_leve;
    let mut filter = builder.filter(None, LevelFilter::from_str(log_level).unwrap());
    filter.init();
    Ok(())
}



pub fn build_redis_pool(config: &AppConfig) -> Pool {
    // 从应用配置中获取 Redis URL
    let mut cfg = deadpool_redis::Config::from_url(config.redis.url.clone());

    // 设置连接池的配置参数
    cfg.pool = Some(PoolConfig {
        max_size: 16, // 最大连接数
        timeouts: Default::default(), // 使用默认的超时时间
        queue_mode: Default::default(), // 使用默认的队列模式
    });
    // 创建并返回连接池
    cfg.create_pool(Some(deadpool_redis::Runtime::Tokio1))
        .expect("Failed to create Redis connection pool")
}

pub async  fn init_mongo_db(config:&AppConfig) -> Database
{
    let client_options = ClientOptions::parse(config.database.url.clone()).await.expect("MongoDB URI ERROR");
    // 创建 MongoDB 客户端
    let client = Client::with_options(client_options).expect("CLIENT MongoDB ERROR");
    // 获取数据库句柄（例如，名为 "mydb" 的数据库）
    client.database(&config.database.db_name)
}

