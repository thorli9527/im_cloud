use common::config::AppConfig;
use common::query_builder::PageInfo;
use config::Config;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use utoipa::ToSchema;
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



#[derive(Serialize, ToSchema)]
pub struct ResultResponse<T: Serialize> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}
impl<T: Serialize> ResultResponse<T> {
    /// 成功响应，带数据
    pub fn ok(data: T) -> Self {
        ResultResponse {
            success: true,
            message: None,
            data: Some(data),
        }
    }

    /// 成功响应，自定义消息 + 可选数据
    pub fn ok_msg(message: impl Into<String>, data: Option<T>) -> Self {
        ResultResponse {
            success: true,
            message: Some(message.into()),
            data,
        }
    }

    /// 失败响应，附带错误消息
    pub fn err(message: impl Into<String>) -> Self {
        ResultResponse {
            success: false,
            message: Some(message.into()),
            data: None,
        }
    }
}
