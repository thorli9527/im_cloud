use common::query_builder::PageInfo;
use config::Config;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt::Debug;
use common::config::AppConfig;

#[derive(Debug,Clone)]
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

#[derive(Debug, Serialize, Deserialize,Clone)]
#[serde(rename_all = "camelCase")]
pub struct PageResult<T>
{
    pub total: i64,
    pub data: Vec<T>,
    pub page_info: PageInfo,
}

pub fn result() -> Value {
    serde_json::json!({"success":true})
}

pub fn result_error_msg(msg: String) -> Value {
    serde_json::json!({"success":false,"msg":msg})
}

pub fn result_warn_msg(msg: String) -> Value {
    serde_json::json!({"success":true,"msg":msg})
}
pub fn result_list<T: Serialize + Debug>(list: Vec<T>) -> Value {
    let value = serde_json::json!({"success":true,"data":list});
    return value;
}
pub fn result_page<T: Serialize + Debug>(page: PageResult<T>) -> Value {
    return serde_json::json!({"success":true,"data":{"list":page.data,"total":page.total,"page":page.page_info}});
}
pub fn result_data<T: Serialize + Debug>(data: T) -> Value {
    return serde_json::json!({"success":true,"data":data});
}
