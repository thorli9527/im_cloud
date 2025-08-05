use common::UserId;
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
#[derive(Clone, Debug)]
pub struct OnLineMessageEntity {
    pub message_id: u64,
    pub uid: UserId,
    pub send_group_status: bool,
}
