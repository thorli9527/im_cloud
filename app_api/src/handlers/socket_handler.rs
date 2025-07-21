use actix_web::{get, web, HttpRequest, HttpResponse, Responder};
use prost::bytes::buf::Reader;
use serde::Serialize;
use serde_json::json;
use common::errors::AppError;
use common::util::common_utils::hash_index;
use crate::handlers::common_handler::status;
use crate::protocol::rpc_arb_models::{NodeType, QueryNodeReq};
use crate::result::{result, result_list};
use crate::service::arb_client::ArbClient;

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
    let ip = req.peer_addr().unwrap().ip().to_string(); // fallback

    let mut arb_client= ArbClient::new().await;
    let query = QueryNodeReq { node_type: NodeType::SocketNode as i32};
    let list = arb_client.client.list_all_nodes(query).await?.into_inner();
    let i = list.nodes.len() as i32;
    let index=hash_index(&ip, i);
    let address=list.nodes[index as usize].socket_addr.clone();
    Ok(HttpResponse::Ok().json(SocketAddrResponse {address: address.unwrap()}))
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