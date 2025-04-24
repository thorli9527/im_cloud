use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// 代理信息结构体，表示一个接入系统或下级代理的配置信息
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub struct AgentInfo {
    /// 代理 ID，唯一标识
    pub id: String,

    /// 代理名称
    pub name: String,

    /// 备注信息
    pub remark: String,

    /// 应用 Key，用于身份验证
    pub app_key: String,

    /// 应用密钥（私密信息），用于签名或鉴权
    pub app_secret: String,

    /// 是否启用该代理（true 表示启用）
    pub enable: bool,

    /// 过期时间（时间戳，单位：秒）
    pub end_time: i64,
    /// 创建时间（Unix 秒时间戳）
    pub create_time: i64,
    /// 最后更新时间（Unix 秒时间戳）
    pub update_time: i64,                    
}
