use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

/// 用户性别枚举
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub enum Gender {
    #[default]
    Unknown, // 未知
    Male,   // 男性
    Female, // 女性
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub enum FriendPolicy {
    #[default]
    AllowAny, // 允许任何人添加好友
    NeedConfirm, // 需要验证
    DenyAny,     // 拒绝任何人添加
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub struct ClientInfo {
    pub id: String,                                     // 用户id
    pub name: String,
    pub username:Option<String>,//用户名
    pub password: Option<String>,                               //密码
    pub uid: String,                                    //客户端用户id
    pub enable: bool,                                   //用户启用状态                   封号
    pub lock: bool,                                     //用户锁定 不能登录
    pub agent_id: String,                               // 商户id
    pub agent_id_uid: String,                           // 商户ID+uid 组合唯一标识
    pub avatar: Option<String>,                         // 头像 URL
    pub allow_add_friend: FriendPolicy,                 // 加好友策略
    pub profile_fields: HashMap<String, String>,        // 用户基本信息
    pub extend_fields: Option<HashMap<String, String>>, // 字段扩展
    pub message_expired_at: Option<i64>,                //禁言时间
    pub message_status: bool,                           //禁言状态 true 禁止
    /// 创建时间（Unix 秒时间戳）
    pub create_time: i64,
    /// 最后更新时间（Unix 秒时间戳）
    pub update_time: i64,
}
