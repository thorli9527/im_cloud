// This file is @generated by prost-build.
/// ======================================
/// 💠 消息内容结构（oneof 类型）
/// ======================================
/// 使用 oneof 定义不同类型的消息内容，确保消息类型的互斥性
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MessageContent {
    #[prost(
        oneof = "message_content::Content",
        tags = "1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21"
    )]
    pub content: ::core::option::Option<message_content::Content>,
}
/// Nested message and enum types in `MessageContent`.
pub mod message_content {
    #[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Content {
        /// 文本消息：纯文本内容
        #[prost(message, tag = "1")]
        Text(super::TextContent),
        /// 图片消息：图片文件
        #[prost(message, tag = "2")]
        Image(super::ImageContent),
        /// 音频消息：语音或音乐
        #[prost(message, tag = "3")]
        Audio(super::AudioContent),
        /// 视频消息：视频文件
        #[prost(message, tag = "4")]
        Video(super::VideoContent),
        /// 位置消息：地理位置信息
        #[prost(message, tag = "5")]
        Location(super::LocationContent),
        /// 文件消息：任意文件类型
        #[prost(message, tag = "6")]
        File(super::FileContent),
        /// 音视频通话信令：通话控制消息
        #[prost(message, tag = "7")]
        AvCall(super::AvCallContent),
        /// 自定义消息：开发者自定义结构
        #[prost(message, tag = "8")]
        Custom(super::CustomContent),
        /// 表情消息：emoji 表情
        #[prost(message, tag = "9")]
        Emoji(super::EmojiContent),
        /// 撤回消息：消息撤回通知
        #[prost(message, tag = "10")]
        Revoke(super::RevokeContent),
        /// 转发消息：消息转发
        #[prost(message, tag = "11")]
        Forward(super::ForwardContent),
        /// 引用回复消息：回复特定消息
        #[prost(message, tag = "12")]
        Quote(super::QuoteContent),
        /// HTML 卡片：富文本内容
        #[prost(message, tag = "13")]
        Html(super::HtmlContent),
        /// VOIP 通话记录：通话日志
        #[prost(message, tag = "14")]
        Voip(super::VoipContent),
        /// 通知消息：系统通知
        #[prost(message, tag = "15")]
        Notification(super::NotificationContent),
        /// 系统消息：系统级消息
        #[prost(message, tag = "16")]
        System(super::SystemContent),
        /// 提醒消息：提醒事项
        #[prost(message, tag = "17")]
        Reminder(super::ReminderContent),
        /// 群组事件：群组相关事件
        #[prost(message, tag = "18")]
        GroupEvent(super::GroupEventContent),
        /// 名片消息：联系人分享
        #[prost(message, tag = "19")]
        ContactCard(super::ContactCardContent),
        /// 投票消息：投票内容
        #[prost(message, tag = "20")]
        Vote(super::VoteContent),
        /// 红包消息：红包内容
        #[prost(message, tag = "21")]
        RedEnvelope(super::RedEnvelopeContent),
    }
}
/// ===============================
/// 📄 文本消息
/// ===============================
/// 支持纯文本和富文本格式，包含内联实体（链接、@用户、话题等）
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TextContent {
    /// 文本主体内容：消息的文本内容
    #[prost(string, tag = "1")]
    pub text: ::prost::alloc::string::String,
    /// 富文本实体，如链接/@用户/话题：文本中的特殊元素
    #[prost(message, repeated, tag = "2")]
    pub entities: ::prost::alloc::vec::Vec<InlineEntity>,
}
/// 内联实体：用于在文本中标记特殊元素
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct InlineEntity {
    /// 起始位置（UTF-8 字符索引）：实体在文本中的开始位置
    #[prost(int32, tag = "1")]
    pub start: i32,
    /// 结束位置（不含）：实体在文本中的结束位置
    #[prost(int32, tag = "2")]
    pub end: i32,
    /// 类型：link / mention / hashtag：实体的类型
    #[prost(string, tag = "3")]
    pub r#type: ::prost::alloc::string::String,
    /// 附加值：URL、用户ID等：实体的具体值
    #[prost(string, tag = "4")]
    pub value: ::prost::alloc::string::String,
}
/// ===============================
/// 🖼️ 图片消息
/// ===============================
/// 包含图片的完整信息，支持原图和缩略图
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ImageContent {
    /// 原图 URL：图片的完整地址
    #[prost(string, tag = "1")]
    pub url: ::prost::alloc::string::String,
    /// 缩略图 URL：图片的缩略图地址
    #[prost(string, tag = "2")]
    pub thumbnail_url: ::prost::alloc::string::String,
    /// 宽度（像素）：图片的宽度
    #[prost(int32, tag = "3")]
    pub width: i32,
    /// 高度（像素）：图片的高度
    #[prost(int32, tag = "4")]
    pub height: i32,
    /// 格式（如 jpg/png）：图片的文件格式
    #[prost(string, tag = "5")]
    pub format: ::prost::alloc::string::String,
    /// 文件大小（字节）：图片文件的大小
    #[prost(int64, tag = "6")]
    pub size: i64,
}
/// ===============================
/// 🔊 音频消息
/// ===============================
/// 支持语音聊天和音乐播放，包含时长和格式信息
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct AudioContent {
    /// 音频文件 URL：音频文件的地址
    #[prost(string, tag = "1")]
    pub url: ::prost::alloc::string::String,
    /// 时长（秒）：音频的播放时长
    #[prost(int32, tag = "2")]
    pub duration: i32,
    /// 格式：音频文件格式（如 mp3/wav）
    #[prost(string, tag = "3")]
    pub format: ::prost::alloc::string::String,
    /// 文件大小（字节）：音频文件的大小
    #[prost(int64, tag = "4")]
    pub size: i64,
    /// 是否语音（vs 音乐类音频）：true表示语音，false表示音乐
    #[prost(bool, tag = "5")]
    pub is_voice: bool,
}
/// ===============================
/// 🎞️ 视频消息
/// ===============================
/// 包含视频文件和封面图，支持播放控制
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct VideoContent {
    /// 视频文件 URL：视频文件的地址
    #[prost(string, tag = "1")]
    pub url: ::prost::alloc::string::String,
    /// 时长（秒）：视频的播放时长
    #[prost(int32, tag = "2")]
    pub duration: i32,
    /// 封面图 URL：视频的封面图片
    #[prost(string, tag = "3")]
    pub cover_url: ::prost::alloc::string::String,
    /// 宽度（像素）：视频的宽度
    #[prost(int32, tag = "4")]
    pub width: i32,
    /// 高度（像素）：视频的高度
    #[prost(int32, tag = "5")]
    pub height: i32,
    /// 格式：视频文件格式（如 mp4/avi）
    #[prost(string, tag = "6")]
    pub format: ::prost::alloc::string::String,
    /// 文件大小（字节）：视频文件的大小
    #[prost(int64, tag = "7")]
    pub size: i64,
}
/// ===============================
/// 📍 位置消息
/// ===============================
/// 包含地理位置信息，支持地址描述和地图显示
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct LocationContent {
    /// 纬度：地理位置的纬度坐标
    #[prost(double, tag = "1")]
    pub latitude: f64,
    /// 经度：地理位置的经度坐标
    #[prost(double, tag = "2")]
    pub longitude: f64,
    /// 地址描述：位置的文字描述
    #[prost(string, tag = "3")]
    pub address: ::prost::alloc::string::String,
    /// 地点名称：具体的地点名称
    #[prost(string, tag = "4")]
    pub poi_name: ::prost::alloc::string::String,
    /// 缩略图 URL：位置的地图缩略图
    #[prost(string, tag = "5")]
    pub thumbnail_url: ::prost::alloc::string::String,
}
/// ===============================
/// 📁 文件消息
/// ===============================
/// 支持任意文件类型，包含文件信息和图标
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct FileContent {
    /// 文件 URL：文件的下载地址
    #[prost(string, tag = "1")]
    pub url: ::prost::alloc::string::String,
    /// 文件名：文件的显示名称
    #[prost(string, tag = "2")]
    pub name: ::prost::alloc::string::String,
    /// 文件大小（字节）：文件的大小
    #[prost(int64, tag = "3")]
    pub size: i64,
    /// 文件类型：文件的 MIME 类型
    #[prost(string, tag = "4")]
    pub file_type: ::prost::alloc::string::String,
    /// 图标 URL：文件类型的图标
    #[prost(string, tag = "5")]
    pub icon_url: ::prost::alloc::string::String,
}
/// ===============================
/// 📞 音视频通话信令
/// ===============================
/// 用于音视频通话的控制信令，包含通话状态和参与者信息
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct AvCallContent {
    /// 通话 ID：通话的唯一标识
    #[prost(string, tag = "1")]
    pub call_id: ::prost::alloc::string::String,
    /// 发起者 ID：通话的发起用户
    #[prost(string, tag = "2")]
    pub initiator_id: ::prost::alloc::string::String,
    /// 参与者 ID 列表：通话的所有参与者
    #[prost(string, repeated, tag = "3")]
    pub participant_ids: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    /// 通话操作：具体的通话动作
    #[prost(enumeration = "av_call_content::CallAction", tag = "4")]
    pub action: i32,
    /// 通话类型：通话的类型
    #[prost(enumeration = "av_call_content::CallType", tag = "5")]
    pub r#type: i32,
    /// 时间戳：操作的时间
    #[prost(int64, tag = "6")]
    pub timestamp: i64,
    /// 时长（秒）：通话的持续时间
    #[prost(int32, tag = "7")]
    pub duration: i32,
}
/// Nested message and enum types in `AVCallContent`.
pub mod av_call_content {
    #[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[derive(
        Clone,
        Copy,
        Debug,
        PartialEq,
        Eq,
        Hash,
        PartialOrd,
        Ord,
        ::prost::Enumeration
    )]
    #[repr(i32)]
    pub enum CallAction {
        /// 未知操作
        Unknown = 0,
        /// 邀请：发起通话邀请
        Invite = 1,
        /// 接受：接受通话邀请
        Accept = 2,
        /// 拒绝：拒绝通话邀请
        Reject = 3,
        /// 取消：取消通话
        Cancel = 4,
        /// 结束：结束通话
        End = 5,
        /// 超时：通话超时
        Timeout = 6,
    }
    impl CallAction {
        /// String value of the enum field names used in the ProtoBuf definition.
        ///
        /// The values are not transformed in any way and thus are considered stable
        /// (if the ProtoBuf definition does not change) and safe for programmatic use.
        pub fn as_str_name(&self) -> &'static str {
            match self {
                Self::Unknown => "UNKNOWN",
                Self::Invite => "INVITE",
                Self::Accept => "ACCEPT",
                Self::Reject => "REJECT",
                Self::Cancel => "CANCEL",
                Self::End => "END",
                Self::Timeout => "TIMEOUT",
            }
        }
        /// Creates an enum from field names used in the ProtoBuf definition.
        pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
            match value {
                "UNKNOWN" => Some(Self::Unknown),
                "INVITE" => Some(Self::Invite),
                "ACCEPT" => Some(Self::Accept),
                "REJECT" => Some(Self::Reject),
                "CANCEL" => Some(Self::Cancel),
                "END" => Some(Self::End),
                "TIMEOUT" => Some(Self::Timeout),
                _ => None,
            }
        }
    }
    #[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[derive(
        Clone,
        Copy,
        Debug,
        PartialEq,
        Eq,
        Hash,
        PartialOrd,
        Ord,
        ::prost::Enumeration
    )]
    #[repr(i32)]
    pub enum CallType {
        /// 音频通话：仅语音通话
        Audio = 0,
        /// 视频通话：音视频通话
        Video = 1,
    }
    impl CallType {
        /// String value of the enum field names used in the ProtoBuf definition.
        ///
        /// The values are not transformed in any way and thus are considered stable
        /// (if the ProtoBuf definition does not change) and safe for programmatic use.
        pub fn as_str_name(&self) -> &'static str {
            match self {
                Self::Audio => "AUDIO",
                Self::Video => "VIDEO",
            }
        }
        /// Creates an enum from field names used in the ProtoBuf definition.
        pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
            match value {
                "AUDIO" => Some(Self::Audio),
                "VIDEO" => Some(Self::Video),
                _ => None,
            }
        }
    }
}
/// ===============================
/// 💠 自定义结构化消息
/// ===============================
/// 支持开发者自定义的消息结构，通常以 JSON 格式承载
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CustomContent {
    /// 自定义类型：消息的自定义类型标识
    #[prost(string, tag = "1")]
    pub custom_type: ::prost::alloc::string::String,
    /// JSON 载荷：自定义消息的 JSON 数据
    #[prost(string, tag = "2")]
    pub json_payload: ::prost::alloc::string::String,
}
/// ===============================
/// 😄 表情消息
/// ===============================
/// 支持标准 emoji 和自定义表情
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct EmojiContent {
    /// 表情类型：标准 emoji 类型
    #[prost(enumeration = "EmojiType", tag = "1")]
    pub emoji: i32,
    /// 自定义表情 URL：自定义表情的图片地址
    #[prost(string, tag = "2")]
    pub custom_emoji_url: ::prost::alloc::string::String,
}
/// ===============================
/// ⛔ 撤回消息
/// ===============================
/// 用于通知消息撤回，包含撤回的目标消息信息
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RevokeContent {
    /// 目标消息 ID：被撤回的消息ID
    #[prost(uint64, tag = "1")]
    pub target_message_id: u64,
    /// 操作者 ID：执行撤回操作的用户
    #[prost(string, tag = "2")]
    pub operator_id: ::prost::alloc::string::String,
    /// 撤回时间：撤回操作的时间戳
    #[prost(int64, tag = "3")]
    pub revoke_time: i64,
}
/// ===============================
/// 📤 转发消息
/// ===============================
/// 用于消息转发，包含原消息的基本信息
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ForwardContent {
    /// 原消息 ID：被转发消息的ID
    #[prost(uint64, tag = "1")]
    pub original_message_id: u64,
    /// 原发送者 ID：原消息的发送者
    #[prost(string, tag = "2")]
    pub original_sender_id: ::prost::alloc::string::String,
    /// 原消息类型：原消息的类型
    #[prost(enumeration = "super::super::common::ContentType", tag = "3")]
    pub original_type: i32,
    /// 摘要：转发的摘要信息
    #[prost(string, tag = "4")]
    pub summary: ::prost::alloc::string::String,
}
/// ===============================
/// 📌 引用回复消息
/// ===============================
/// 用于回复特定消息，包含被引用消息的信息
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct QuoteContent {
    /// 被引用消息 ID：被回复消息的ID
    #[prost(uint64, tag = "1")]
    pub quoted_message_id: u64,
    /// 被引用内容预览：被回复消息的预览
    #[prost(string, tag = "2")]
    pub quoted_content_preview: ::prost::alloc::string::String,
    /// 引用文本：回复的文本内容
    #[prost(string, tag = "3")]
    pub quote_text: ::prost::alloc::string::String,
}
/// ===============================
/// 🌐 HTML 卡片
/// ===============================
/// 用于富文本内容，支持网页链接和预览
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct HtmlContent {
    /// 标题：卡片的标题
    #[prost(string, tag = "1")]
    pub title: ::prost::alloc::string::String,
    /// URL：链接地址
    #[prost(string, tag = "2")]
    pub url: ::prost::alloc::string::String,
    /// 预览：内容的预览文本
    #[prost(string, tag = "3")]
    pub preview: ::prost::alloc::string::String,
}
/// ===============================
/// 📞 VOIP 通话记录
/// ===============================
/// 用于记录通话历史，包含通话的基本信息
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct VoipContent {
    /// 主叫 ID：发起通话的用户
    #[prost(string, tag = "1")]
    pub caller_id: ::prost::alloc::string::String,
    /// 被叫 ID：接收通话的用户
    #[prost(string, tag = "2")]
    pub callee_id: ::prost::alloc::string::String,
    /// 通话时长（秒）：通话的持续时间
    #[prost(int64, tag = "3")]
    pub duration: i64,
    /// 通话状态：通话的结果状态
    #[prost(string, tag = "4")]
    pub status: ::prost::alloc::string::String,
}
/// ===============================
/// 🔔 通知消息
/// ===============================
/// 用于系统通知，包含标题、内容和元数据
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct NotificationContent {
    /// 标题：通知的标题
    #[prost(string, tag = "1")]
    pub title: ::prost::alloc::string::String,
    /// 内容：通知的主体内容
    #[prost(string, tag = "2")]
    pub body: ::prost::alloc::string::String,
    /// 元数据：通知的附加信息
    #[prost(map = "string, string", tag = "3")]
    pub metadata: ::std::collections::HashMap<
        ::prost::alloc::string::String,
        ::prost::alloc::string::String,
    >,
}
/// ===============================
/// ⚙️ 系统消息
/// ===============================
/// 用于系统级消息，包含系统代码和内容
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SystemContent {
    /// 内容：系统消息的内容
    #[prost(string, tag = "1")]
    pub content: ::prost::alloc::string::String,
    /// 代码：系统消息的代码标识
    #[prost(string, tag = "2")]
    pub code: ::prost::alloc::string::String,
}
/// ===============================
/// ⏰ 提醒事项
/// ===============================
/// 用于提醒功能，包含提醒文本和时间
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ReminderContent {
    /// 文本：提醒的内容
    #[prost(string, tag = "1")]
    pub text: ::prost::alloc::string::String,
    /// 提醒时间：提醒触发的时间戳
    #[prost(int64, tag = "2")]
    pub remind_at: i64,
}
/// ===============================
/// 👪 群组事件
/// ===============================
/// 用于群组相关事件，包含群组信息和操作者
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GroupEventContent {
    /// 群组 ID：事件相关的群组
    #[prost(string, tag = "1")]
    pub group_id: ::prost::alloc::string::String,
    /// 事件：事件的具体描述
    #[prost(string, tag = "2")]
    pub event: ::prost::alloc::string::String,
    /// 操作者 ID：执行操作的用户
    #[prost(string, tag = "3")]
    pub operator_id: ::prost::alloc::string::String,
}
/// ===============================
/// 📇 名片消息
/// ===============================
/// 用于分享联系人信息，包含用户的基本信息
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ContactCardContent {
    /// 目标 ID：被分享用户的ID
    #[prost(string, tag = "1")]
    pub target_id: ::prost::alloc::string::String,
    /// 显示名称：用户的显示名称
    #[prost(string, tag = "2")]
    pub display_name: ::prost::alloc::string::String,
    /// 头像 URL：用户的头像地址
    #[prost(string, tag = "3")]
    pub avatar_url: ::prost::alloc::string::String,
    /// 卡片类型：名片的类型（用户/群组）
    #[prost(string, tag = "4")]
    pub card_type: ::prost::alloc::string::String,
}
/// ===============================
/// 📊 投票消息
/// ===============================
/// 用于群组投票功能，包含投票选项和结果
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct VoteContent {
    /// 主题：投票的主题
    #[prost(string, tag = "1")]
    pub topic: ::prost::alloc::string::String,
    /// 选项：投票的选项列表
    #[prost(string, repeated, tag = "2")]
    pub options: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    /// 结果：投票结果统计
    #[prost(map = "string, int32", tag = "3")]
    pub result: ::std::collections::HashMap<::prost::alloc::string::String, i32>,
    /// 多选：是否允许多选
    #[prost(bool, tag = "4")]
    pub multi_choice: bool,
}
/// ===============================
/// 💰 红包消息
/// ===============================
/// 用于红包功能，包含红包金额和状态
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RedEnvelopeContent {
    /// 发送者 ID：红包发送者
    #[prost(string, tag = "1")]
    pub sender_id: ::prost::alloc::string::String,
    /// 金额：红包金额（分）
    #[prost(int32, tag = "2")]
    pub amount: i32,
    /// 祝福语：红包的祝福语
    #[prost(string, tag = "3")]
    pub blessing: ::prost::alloc::string::String,
    /// 是否已领取：红包是否已被领取
    #[prost(bool, tag = "4")]
    pub claimed: bool,
}
/// ======================================
/// ✂️ Segment - 消息段结构（用于复合内容）
/// ======================================
/// 表示一条消息中的一个独立段（如文本段、图片段等），支持排序、编辑、标记等
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Segment {
    /// 消息段内容（如文本、图片等，使用 oneof 封装）：段的具体内容
    #[prost(message, optional, tag = "1")]
    pub body: ::core::option::Option<MessageContent>,
    /// 段唯一标识（客户端生成或服务端补全）：段的唯一ID
    #[prost(string, tag = "2")]
    pub segment_id: ::prost::alloc::string::String,
    /// 消息内顺序编号（用于前端渲染排序）：段在消息中的顺序
    #[prost(uint64, tag = "3")]
    pub seq_in_msg: u64,
    /// 是否为编辑后的段落（true 表示被修改）：段的编辑状态
    #[prost(bool, tag = "4")]
    pub edited: bool,
    /// 是否允许客户端渲染该段（false 可用于隐藏草稿等）：段的可见性
    #[prost(bool, tag = "5")]
    pub visible: bool,
    /// 通用扩展字段（以字符串键值对存储 JSON 扁平数据）：段的元数据
    #[prost(map = "string, string", tag = "6")]
    pub metadata: ::std::collections::HashMap<
        ::prost::alloc::string::String,
        ::prost::alloc::string::String,
    >,
}
/// ======================================
/// 📨 顶层消息结构
/// ======================================
/// 定义了消息的基本框架，包含发送者、接收者、时间等元数据
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Content {
    /// 唯一消息 ID（客户端生成或服务端补全）：消息的唯一标识
    #[prost(uint64, optional, tag = "1")]
    pub message_id: ::core::option::Option<u64>,
    /// 消息发送方：发送消息的用户ID
    #[prost(string, tag = "2")]
    pub sender_id: ::prost::alloc::string::String,
    /// 消息接收方（单聊为对方 ID，群聊为群 ID）：接收消息的目标
    #[prost(string, tag = "3")]
    pub receiver_id: ::prost::alloc::string::String,
    /// 消息发送时间（毫秒时间戳）：消息创建的时间
    #[prost(int64, tag = "4")]
    pub timestamp: i64,
    /// 主消息类型（用于快速渲染判断）：消息的主要类型
    #[prost(enumeration = "super::super::common::ContentType", tag = "5")]
    pub message_type: i32,
    /// 消息所属会话类型（单聊/群聊）：消息的会话场景
    #[prost(enumeration = "ChatScene", tag = "6")]
    pub scene: i32,
    /// 多段复合内容（如文本 + 图片）：消息的具体内容
    #[prost(message, repeated, tag = "10")]
    pub contents: ::prost::alloc::vec::Vec<MessageContent>,
}
/// ======================================
/// 💬 会话场景类型
/// ======================================
/// 用于标识消息所属的会话类型，帮助客户端进行不同的渲染和处理
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum ChatScene {
    /// 默认未知场景（防御值）
    ChatUnknown = 0,
    /// 单人会话：用户与用户之间的私聊
    Single = 1,
    /// 群聊会话：群组内的多人聊天
    Group = 2,
}
impl ChatScene {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            Self::ChatUnknown => "CHAT_UNKNOWN",
            Self::Single => "SINGLE",
            Self::Group => "GROUP",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "CHAT_UNKNOWN" => Some(Self::ChatUnknown),
            "SINGLE" => Some(Self::Single),
            "GROUP" => Some(Self::Group),
            _ => None,
        }
    }
}
/// ======================================
/// 😄 Emoji 类型定义（标准 + 自定义）
/// ======================================
/// 定义了系统中支持的 emoji 类型，包括标准 emoji 和自定义表情
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum EmojiType {
    EmojiUnknown = 0,
    /// 微笑表情
    Smile = 1,
    /// 咧嘴笑表情
    Grin = 2,
    /// 流泪表情
    Tears = 3,
    /// 吐舌头表情
    StuckOutTongue = 4,
    /// 鼓掌表情
    Clap = 25,
    /// 便便表情
    Poop = 28,
    /// 爱心表情
    Heart = 21,
    /// ... 可继续扩展标准 Emoji
    ///
    /// 自定义表情（通过图片 URL 指定）
    CustomEmoji = 1000,
}
impl EmojiType {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            Self::EmojiUnknown => "EMOJI_UNKNOWN",
            Self::Smile => "SMILE",
            Self::Grin => "GRIN",
            Self::Tears => "TEARS",
            Self::StuckOutTongue => "STUCK_OUT_TONGUE",
            Self::Clap => "CLAP",
            Self::Poop => "POOP",
            Self::Heart => "HEART",
            Self::CustomEmoji => "CUSTOM_EMOJI",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "EMOJI_UNKNOWN" => Some(Self::EmojiUnknown),
            "SMILE" => Some(Self::Smile),
            "GRIN" => Some(Self::Grin),
            "TEARS" => Some(Self::Tears),
            "STUCK_OUT_TONGUE" => Some(Self::StuckOutTongue),
            "CLAP" => Some(Self::Clap),
            "POOP" => Some(Self::Poop),
            "HEART" => Some(Self::Heart),
            "CUSTOM_EMOJI" => Some(Self::CustomEmoji),
            _ => None,
        }
    }
}
