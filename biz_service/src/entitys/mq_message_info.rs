use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MessageSegment {
    /// 纯文本内容
    Text {
        text: String,
    },

    /// 图片消息
    Image {
        url: String,
        width: Option<u32>,
        height: Option<u32>,
        name: Option<String>,      // 原始文件名
        size: Option<u64>,         // 文件大小（字节）
    },

    /// 文件消息（含文档、PDF、压缩包等）
    File {
        url: String,
        name: String,
        size: u64,
        mime_type: Option<String>,
    },

    /// 表情 / Emoji
    Emoji {
        name: String,              // 表情名称（如 smile）
        unicode: String,           // 😀、😎 等
        src: Option<String>,       // 自定义表情资源 URL（可选）
    },

    /// @提及
    Mention {
        user_id: i64,
        username: String,
    },

    /// 引用其他消息（message_id + preview）
    Quote {
        message_id: i64,
        preview: String,
    },

    /// 音频消息
    Audio {
        url: String,
        duration: u32,             // 播放时长（秒）
        size: u64,
    },

    /// 视频消息
    Video {
        url: String,
        duration: u32,
        width: u32,
        height: u32,
        size: u64,
        cover_url: Option<String>, // 视频封面
    },

    /// 系统提示（如入群、撤回提示等）
    SystemTip {
        text: String,
    },

    /// HTML 富文本片段（不建议客户端输入，仅系统输出）
    Html {
        html: String,
    },

    /// 自定义消息类型（保留扩展）
    Custom {
        name: String,             // 自定义类型标识
        payload: serde_json::Value,
    },
}


/// 群聊消息类型，用于顶层标记消息所属主类别
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    /// 文本
    Text,
    /// 图片
    Image,
    /// 音频
    Audio,
    /// 视频
    Video,
    /// 位置
    Location,
    /// 文件
    File,
    /// 音视频通话
    AvCall,
    /// 自定义消息
    Custom,
    /// 头像
    Emoji,
    /// 撤回
    Revoke,
    /// 转发
    Forward,
    /// 引用消息
    Quote,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserMessage{
    /// 全局唯一消息 ID（如雪花 ID）
    pub id: String,
    /// 所属商户
    pub agent_id:String,
    /// 所属用户id
    pub user_id: String,
    /// 发送者用户 ID
    pub sender_id: String,
    /// 消息类型标记（方便数据库索引/前端展示）
    pub message_type: MessageType,
    /// 消息复合内容（支持结构化消息段）
    pub content: GroupMessageContent,
    pub created_time: i64,                    // 创建时间（Unix 秒时间戳）
    pub updated_time: i64,                    // 最后更新时间（Unix 秒时间戳）
    /// 对应序号（用于顺序拉取）
    pub seq: i64,
    /// 是否被撤回
    pub revoked: bool,
    /// 是否为系统消息（可用于区分人工发送和自动提示）
    pub is_system: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GroupMessage {
    /// 全局唯一消息 ID（如雪花 ID）
    pub id: String,
    /// 所属商户
    pub agent_id:String,
    /// 所属群组 ID
    pub group_id: String,
    /// 发送者用户 ID
    pub sender_id: String,
    /// 消息类型标记（方便数据库索引/前端展示）
    pub message_type: MessageType,
    /// 消息复合内容（支持结构化消息段）
    pub content: GroupMessageContent,
    /// 创建时间（Unix 秒时间戳）
    pub create_time: i64,
    /// 最后更新时间（Unix 秒时间戳）
    pub update_time: i64,
    /// 群内顺序号（用于顺序拉取）
    pub seq: i64,
    /// 是否被撤回
    pub revoked: bool,
    /// 是否为系统消息（可用于区分人工发送和自动提示）
    pub is_system: bool,

}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GroupMessageContent {
    pub segments: Vec<MessageSegment>,
}