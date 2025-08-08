use crate::handlers::common_handler::status;
use actix_web::{get, web, HttpRequest, HttpResponse, Responder};
use biz_service::protocol::rpc::arb_models::NodeType;
use common::errors::AppError;
use common::util::common_utils::hash_index;
use serde::Serialize;
use biz_service::kafka_util::node_util::NodeUtil;

#[derive(Serialize)]
pub struct SocketAddrResponse {
    pub address: String,
}
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(status);
}
#[get("/socket/address")]
pub async fn get_socket_address(req: HttpRequest) -> Result<impl Responder, AppError> {
    // Step 1: 获取客户端 IP
    let ip = req
        .peer_addr()
        .map(|addr| addr.ip().to_string())
        .unwrap_or_else(|| "127.0.0.1".to_string()); // fallback

    let node_util = NodeUtil::get();
    let node_list = node_util.await.get_list(NodeType::SocketNode);
    let i = node_list.len() as i32;
    let index = hash_index(&ip, i);
    let address = node_list[index as usize].clone();
    Ok(HttpResponse::Ok().json(SocketAddrResponse {
        address: address.node_addr,
    }))
}

fn select_best_region(country: &str) -> Vec<&'static str> {
    match country {
        "CN" => vec!["CN", "HK", "JP"],
        "TW" => vec!["TW", "HK", "JP"],
        "HK" => vec!["HK", "JP", "CN"],
        "JP" => vec!["JP", "HK", "CN"],
        "US" | "CA" => vec!["US", "EU"],
        "GB" | "FR" | "DE" => vec!["EU", "US"],
        _ => vec!["HK", "US"],
    }
}
