use common::config::AppConfig;
use config::Config;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt::Debug;
use std::option::Option;
use utoipa::ToSchema;

#[derive(Debug, Clone)]
pub struct AppState {
    pub config: AppConfig,
}

impl AppState {
    pub fn new() -> Self {
        let config = Config::builder()
            .add_source(config::File::with_name("main-config.toml").required(true))
            .add_source(config::Environment::with_prefix("APP").separator("_"))
            .build()
            .expect("Failed to build configuration");
        let cfg = config.try_deserialize::<AppConfig>().expect("Failed to deserialize configuration");
        Self { config: cfg }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct ApiResponse<T> {
    code: i32,
    message: String,
    data: Option<T>,
}

impl ApiResponse<String> {}


pub struct TokenDto{
    pub token:String
}

pub fn result() -> Value {
    serde_json::json!({"code":200})
}
pub fn result_data<T: Serialize + Debug>(data: T) -> Value {
    return serde_json::json!({"success":true,"data":data});
}

pub fn result_error_msg(msg: &str) -> Value {
    serde_json::json!({"code":false,"msg":msg})
}

pub fn result_warn_msg(msg: &str) -> Value {
    serde_json::json!({"code":true,"msg":msg})
}
pub fn result_list<T: Serialize + Debug>(list: Vec<T>) -> Value {
    let value = serde_json::json!({"code":200,"data":list});
    return value;
}
