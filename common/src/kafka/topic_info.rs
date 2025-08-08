use once_cell::sync::Lazy;

#[derive(Debug, Clone)]
pub struct TopicInfo {
    pub topic_name: String,
    pub partitions: i32,
    pub replicas: i32,
}

impl TopicInfo {
    pub fn new(topic_name: impl Into<String>, partitions: i32, replicas: i32) -> Self {
        Self {
            topic_name: topic_name.into(),
            partitions,
            replicas,
        }
    }
}
/// 在线消息 Topic
pub static ONLINE_TOPIC_INFO: Lazy<TopicInfo> = Lazy::new(|| TopicInfo::new("online", 50, 2));
///消息发送 Topic
pub static MSG_SEND_TOPIC_INFO: Lazy<TopicInfo> = Lazy::new(|| TopicInfo::new("msg-send", 100, 3));
///用户心跳 / 活跃状态更新 Topic
pub static USER_PRESENCE_TOPIC_INFO: Lazy<TopicInfo> =
    Lazy::new(|| TopicInfo::new("user-presence", 30, 2));
