use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// 配置信息结构体，表示系统中的可配置项
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub struct ConfigEntity {
    /// 配置 ID，唯一标识
    pub id: String,

    /// 配置名称
    pub name: String,

    /// 配置类型（枚举，如块大小、连接数等）
    pub config_type: ConfigTypeEnum,

    /// 创建时间（Unix 秒时间戳）
    pub create_time: i64,
    /// 最后更新时间（Unix 秒时间戳）
    pub update_time: i64,
}

/// 配置类型枚举，定义支持的配置项种类
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub enum ConfigTypeEnum {
    /// 默认值：块大小配置（如文件分片大小）
    #[default]
    BlockSize,
}
