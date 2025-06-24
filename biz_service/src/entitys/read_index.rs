use crate::entitys::mq_message_info::ChatTargetType;

pub struct ReadIndex {
    pub from: String,
    pub to: String, // 单聊为 user_id，群聊为 group_id
    pub target_type: ChatTargetType,
    pub read_seq: i64,
    pub updated_at: i64,
}
