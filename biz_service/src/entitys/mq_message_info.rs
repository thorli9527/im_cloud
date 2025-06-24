use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Deserialize, Serialize, ToSchema,Clone)]
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
impl Default for MessageSegment {
    fn default() -> Self {
        MessageSegment::Text {
            text: String::new(),
        }
    }
}
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq,ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ChatTargetType {
    Single = 0,
    Group = 1,
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
    pub from: String,
    ///  ID
    pub to: String,
    /// 消息复合内容（支持结构化消息段）
    pub content: Vec<Segment>,
    pub created_time: i64,                    // 创建时间（Unix 秒时间戳）
    pub updated_time: i64,                    // 最后更新时间（Unix 秒时间戳）
    /// 是否被撤回
    pub revoked: bool,
    /// 是否为系统消息（可用于区分人工发送和自动提示）
    pub is_system: bool,
    /// 是否已发送到 MQ
    pub sync_mq_status: bool,
    /// 是否已送达客户端（如 WebSocket 成功推送）
    pub delivered: bool,
    /// 阅读时间戳（Unix 秒时间戳，可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_time: Option<i64>,
}
#[derive(Debug, Deserialize, Serialize, ToSchema,Validate)]
#[serde(rename_all = "camelCase")]
pub struct SegmentDto{
    /// 消息段类型及内容
    #[serde(flatten)]
    pub body: MessageSegment,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GroupMessage {
    /// 全局唯一消息 ID（如雪花 ID）
    pub id: String,
    /// 所属商户
    pub agent_id:String,
    /// 所属群组 ID
    pub to: String,
    /// 发送者用户 ID
    pub from: String,
    /// 消息复合内容（支持结构化消息段）
    pub content: Vec<Segment>,
    /// 创建时间（Unix 秒时间戳）
    pub create_time: i64,
    /// 最后更新时间（Unix 秒时间戳）
    pub update_time: i64,
    /// 群内顺序号（用于顺序拉取）
    pub seq: i64,
    /// 是否已发送到 MQ
    pub sync_mq_status: bool,
    /// 是否被撤回
    pub revoked: bool,
    /// 是否为系统消息（可用于区分人工发送和自动提示）
    pub is_system: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone,Default)]
pub struct Segment{
    /// 消息段类型及内容
    #[serde(flatten)]
    pub body: MessageSegment,
    /// ID，用于局部标记、编辑等
    pub segment_id: String,
    /// 在消息中的顺序编号（用于前端排布）
    pub seq_in_msg: u64,
    /// 是否为编辑后的段
    pub edited: bool,
    /// 是否允许客户端渲染
    pub visible: bool,
    /// 通用扩展字段
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}