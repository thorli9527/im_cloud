use crate::result::ResultResponse;
use actix_session::Session;
use actix_web::{Responder, web};
use biz_service::biz_services::user_service::UserService;
use common::errors::AppError;
use common::repository_util::Repository;
use common::util::common_utils::build_id;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
struct PlatInfo {
    token: String,
    movie_id: String,
}
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate,Default)]
#[serde(rename_all = "camelCase")]

struct PlayItemResult{
    token:String,
    play_key:String
}
//生成播放链接
pub async fn build_play(dto: web::Json<PlatInfo>, user_info: web::Data<UserService>) -> Result<impl Responder, AppError> {
    Ok(web::Json(ResultResponse::ok(vec![PlayItemResult::default()])))
}
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
struct PlayStream {
    token: String,
    play_key: String,
}
//下载播放流
pub async fn play_stream(dto:web::Json<PlayStream>){

}


