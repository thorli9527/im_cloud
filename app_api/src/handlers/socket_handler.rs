use crate::handlers::common_handler::status;
use crate::service::arb_client::ArbClient;
use actix_web::{get, web, HttpRequest, HttpResponse, Responder};
use biz_service::protocol::rpc::arb_models::{NodeType, QueryNodeReq};
use common::errors::AppError;
use common::util::common_utils::hash_index;
use moka::sync::Cache;
use once_cell::sync::Lazy;
use serde::Serialize;
use std::time::Duration;

static SOCKET_ADDR_CACHE: Lazy<Cache<String, String>> =
    Lazy::new(|| Cache::builder().time_to_live(Duration::from_secs(60)).max_capacity(10_000).build());
pub struct SocketHandler;
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
    let ip = req.peer_addr().map(|addr| addr.ip().to_string()).unwrap_or_else(|| "127.0.0.1".to_string()); // fallback

    // Step 2: 查询缓存
    if let Some(cached) = SOCKET_ADDR_CACHE.get(&ip) {
        return Ok(HttpResponse::Ok().json(SocketAddrResponse {
            address: cached,
        }));
    }

    // Step 3: 查询 ARB
    let mut arb_client = ArbClient::new().await;
    let query = QueryNodeReq {
        node_type: NodeType::SocketNode as i32,
    };
    let list = arb_client.client.list_all_nodes(query).await.map_err(|e| AppError::Internal(format!("query arb failed: {}", e)))?.into_inner();

    let i = list.nodes.len() as i32;
    let index = hash_index(&ip, i);
    let address = list.nodes[index as usize].node_addr.clone();

    // Step 4: 写入缓存
    SOCKET_ADDR_CACHE.insert(ip.clone(), address.clone());

    Ok(HttpResponse::Ok().json(SocketAddrResponse {
        address,
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
