use biz_service::protocol::msg::auth::DeviceType;
use common::UserId;
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
#[derive(Clone, Debug)]
pub struct OnLineMessageEntity {
    pub id: String,
    pub message_id: u64,
    pub uid: UserId,
    pub client_id: String,
    pub device_type: DeviceType,
    pub login_time: i64,
    pub send_group_status: bool,
}
