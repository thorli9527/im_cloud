use actix_web::body::BoxBody;
use actix_web::http::header;
use actix_web::{HttpRequest, HttpResponse, Responder};
use serde::Serialize;
use serde_json::Value;
use std::fmt::Debug;
use std::option::Option;
use utoipa::ToSchema;


pub fn result_data<T: Serialize + Debug>(data: T) -> Value {
    return serde_json::json!({"success":true,"data":data});
}

pub fn result_error_msg(msg: &str) -> Value {
    serde_json::json!({"success":false,"msg":msg})
}

pub fn result_warn_msg(msg: &str) -> Value {
    serde_json::json!({"success":true,"msg":msg})
}
#[derive(Serialize, ToSchema)]
pub struct ApiResponse<T> {
    code: i32,
    message: String,
    data: Option<T>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        ApiResponse { code: 0, message: "success".to_string(), data: Some(data) }
    }

    pub fn error(code: i32, msg: impl AsRef<str> + ToString) -> Self {
        ApiResponse { code, message: msg.to_string(), data: None }
    }
}

impl ApiResponse<Value> {
    pub fn json(data: Value) -> Self {
        ApiResponse { code: 0, message: "success".to_string(), data: Some(data) }
    }
}

impl ApiResponse<String> {
    pub fn success_ok() -> Self {
        ApiResponse { code: 0, message: "success".to_string(), data: Option::None }
    }
}

// pub fn result_page<T: Serialize>(page: PageResult<T>) -> ApiResponse<PageResult<T>> {
//     ApiResponse::success(page)
// }

pub fn result() -> ApiResponse<String> {
    ApiResponse::<String>::success_ok()
}

pub fn result_list(json: Value) -> ApiResponse<Value> {
    let json = serde_json::json!({
        "list":json.get("items"),
    });
    ApiResponse::<Value>::json(json)
}

pub fn result_page(json: Value) -> ApiResponse<Value> {
    let json = serde_json::json!({
        "list":json.get("items"),
        "hasNext":json.get("hasNext"),
        "hasPrev":json.get("hasPrev")
    });
    ApiResponse::<Value>::json(json)
}

pub fn result_error(message: impl AsRef<str> + ToString) -> ApiResponse<String> {
    ApiResponse::<String>::error(500, message)
}
impl<T: Serialize> Responder for ApiResponse<T> {
    type Body = BoxBody;

    fn respond_to(self, _req: &HttpRequest) -> HttpResponse<Self::Body> {
        HttpResponse::Ok().insert_header((header::CONTENT_TYPE, "application/json")).json(self)
    }
}
