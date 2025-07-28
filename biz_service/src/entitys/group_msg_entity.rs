use crate::protocol::msg::message::Segment;

/// ======================================
/// 👥 群组消息结构（群聊消息）
/// ======================================
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GroupMsgEntity {
    /// 消息唯一 ID
    #[prost(uint64, tag = "1")]
    pub message_id: u64,
    /// 发送者用户ID
    #[prost(string, tag = "2")]
    pub from: ::prost::alloc::string::String,
    /// 群组ID
    #[prost(string, tag = "3")]
    pub to: ::prost::alloc::string::String,
    /// 消息内容
    #[prost(message, repeated, tag = "4")]
    pub content: ::prost::alloc::vec::Vec<Segment>,
    /// 群内消息序号
    #[prost(int64, tag = "5")]
    pub seq: i64,
    /// 是否撤回
    #[prost(bool, tag = "6")]
    pub revoked: bool,
    /// 是否系统消息
    #[prost(bool, tag = "7")]
    pub is_system: bool,
    /// 是否同步到 MQ
    #[prost(bool, tag = "8")]
    pub sync_mq_status: bool,
    /// 创建时间
    #[prost(int64, tag = "99")]
    pub create_time: i64,
    /// 更新时间
    #[prost(int64, tag = "100")]
    pub update_time: i64,
}
