/// ==========================
/// 👤 客户端用户实体定义
/// ==========================
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
#[derive(Clone, Debug)]
pub struct ClientEntity {
    /// 用户 ID（全局唯一，用于主键或索引）
    pub id: ::prost::alloc::string::String,
    /// 别名
    pub name: ::prost::alloc::string::String,
    /// 用户密码（建议加密存储，仅服务器端处理）
    pub password: ::prost::alloc::string::String,
    /// 客户端用户 ID（可与设备、App 安装等绑定）
    pub uid: ::prost::alloc::string::String,
    /// 邮箱地址（可选，用于验证、找回密码、通知等）
    pub email: ::core::option::Option<::prost::alloc::string::String>,
    /// 手机号码（可选，可用于登录、验证、通知等）
    pub phone: ::core::option::Option<::prost::alloc::string::String>,
    /// 用户首选语言（如 "zh-CN", "en-US" 等）
    pub language: ::core::option::Option<::prost::alloc::string::String>,
    /// 用户头像 URL（可为空字符串表示无头像；建议提供默认头像逻辑）
    pub avatar: ::prost::alloc::string::String,
    /// 加好友策略（如允许任何人、仅限手机号、需要验证等）
    pub allow_add_friend: i32,
    /// 性别枚举（如男、女、保密等）
    pub gender: i32,
    /// 用户类型（如普通用户、机器人、游客、测试账户等）
    pub user_type: i32,
    /// 扩展信息字段（如学校、职业、自定义标签等，键值对形式）
    pub profile_fields: ::std::collections::HashMap<::prost::alloc::string::String, ::prost::alloc::string::String>,
    /// 创建时间（Unix 秒时间戳，用于记录账号创建时间）
    pub create_time: u64,
    /// 最后更新时间（Unix 秒时间戳，用于缓存刷新或数据同步）
    pub update_time: u64,
}
