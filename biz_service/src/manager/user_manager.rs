use crate::entitys::client_entity::ClientEntity;
use crate::protocol::msg::auth::DeviceType;
use anyhow::Result;
use async_trait::async_trait;
use common::{ClientTokenDto, UserId};
use dashmap::DashMap;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::Notify;

/// 全局用户管理器
#[derive(Debug, Clone)]
/// `UserManager` 管理用户在线状态、群组缓存、初始化状态及 Redis 通信。
pub struct UserManager {
    /// Redis 连接池，用于访问用户状态、群组数据、事件队列等。
    pub pool: Arc<common::RedisPool>,

    /// 标记是否已初始化，避免重复初始化。
    /// 使用 Arc + AtomicBool 保证跨线程安全修改。
    pub is_initialized: Arc<AtomicBool>,

    /// 初始化通知器，未完成初始化时异步任务可以 await 等待。
    /// 配合 `is_initialized` 实现任务级初始化阻塞。
    pub init_notify: Arc<Notify>,
    /// 全局用户好友关系映射，使用 DashMap 支持多线程并发访问。
    pub friend_map: Arc<DashMap<String, DashMap<UserId, ()>>>,
}

/// 用户管理核心行为抽象接口
#[async_trait]
pub trait UserManagerOpt: Send + Sync {
    /// 登录用户，将用户标记为在线，并进行必要的缓存更新和事件通知

    /// 将用户标记为在线，并进行必要的缓存更新和事件通知
    async fn online(&self, user_id: &UserId, device_type: &DeviceType) -> Result<()>;
    /// 检查用户是否在线，返回 true 或 false
    async fn is_online(&self, user_id: &UserId) -> Result<bool>;
    /// 检查用户是否所有设备都离线，返回 true 或 false
    async fn is_all_device_offline(&self, user_id: &UserId) -> Result<bool>;
    /// 获取用户在线的设备列表
    async fn get_online_devices(&self, user_id: &UserId) -> Result<Vec<DeviceType>>;
    /// 将用户标记为离线，更新缓存并通知其他服务
    async fn offline(&self, user_id: &UserId, device_type: &DeviceType) -> Result<()>;
    /// 同步指定用户的信息（例如从数据库或Redis加载最新数据到本地缓存）
    async fn sync_user(&self, user: ClientEntity) -> Result<()>;
    /// 移除指定用户缓存
    async fn remove_user(&self, user_id: &UserId) -> Result<()>;
    /// 获取用户的在线状态
    async fn get_user_info(&self, user_id: &UserId) -> Result<Option<ClientEntity>>;

    async fn get_user_info_by_name(&self, name: &str) -> Result<Option<ClientEntity>>;
    /// 构建用户的访问令牌（例如JWT或其他形式的认证令牌）
    async fn build_token(&self, user_id: &UserId, device_type: &DeviceType) -> Result<String>;
    /// 删除用户的访问令牌
    async fn delete_token(&self, token: &str) -> Result<()>;
    /// 验证用户的访问令牌，返回用户ID或错误
    async fn verify_token(&self, token: &str) -> Result<bool>;
    /// 清空某用户所有 token
    async fn clear_tokens_by_user(&self, user_id: &UserId) -> Result<()>;
    /// 查询用户token
    async fn get_token_by_uid_device(&self, user_id: &UserId, device_type: &DeviceType) -> Result<Option<String>>;
    /// 获取用户的访问令牌信息
    async fn get_client_token(&self, token: &str) -> Result<ClientTokenDto>;
    /// 根据令牌查找用户信息
    async fn find_user_by_token(&self, token: &str) -> Result<Option<ClientEntity>>;
    /// 移除好友关系
    /// 检查用户是否是好友关系
    async fn is_friend(&self, user_id: &UserId, friend_id: &UserId) -> Result<bool>;
    /// 获取用户的好友列表
    async fn get_friends(&self, user_id: &UserId) -> Result<Vec<UserId>>;
    /// 拉黑好友
    async fn friend_block(&self, user_id: &UserId, friend_id: &UserId) -> Result<()>;
    /// 拉黑好友-取消
    async fn friend_unblock(&self, user_id: &UserId, friend_id: &UserId) -> Result<()>;
}
include!("user_manager_impl.rs");
