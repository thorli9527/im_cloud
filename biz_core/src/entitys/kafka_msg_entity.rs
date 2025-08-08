pub enum KafkaServerType {
    Socket,
    Group,
}
/// 表示一条 Kafka 消息实体结构，用于封装消息发送的元数据与内容。
pub struct KafkaMsgEntity {
    /// Kafka 主题名称，消息将发送至此主题。
    pub topic: String,
    /// Kafka 消息的 Key，用于消息分区的一致性路由。
    pub key: String,
    /// 实际要发送的消息内容（通常是 JSON、Protobuf、字符串等）。
    pub content: String,
    /// 消息的时间戳（毫秒），通常为发送时间，可用于消息排序或幂等。
    pub timestamp: i64,
    /// 消息是否已成功发送（本地标记，非 Kafka ACK 状态）。
    pub send_status: bool,
    /// 目标服务器类型列表，用于指定消息需要发送到的服务器类型。
    pub send_socket_node: bool,
    /// 群组ID列表，用于指定消息需要发送到的群组。
    pub send_group_node: bool,
    /// Kafka broker 列表，用于指定消息将被发送到哪些 Kafka 节点（用于多集群、定向发送场景）。
    pub broker_list: Vec<String>,
}
