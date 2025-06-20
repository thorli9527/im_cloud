use crate::biz_service::client_service::ClientService;
use crate::entitys::mq_user_action::{UserActionLog, UserActionType};
use crate::manager::common::UserId;
use crate::manager::user_redis_manager::{UserManager, UserManagerOpt};
use anyhow::Result;
use common::errors::AppError;
use common::repository_util::{BaseRepository, Repository};
use common::util::common_utils::as_ref_to_string;
use mongodb::Database;
use once_cell::sync::OnceCell;
use std::sync::Arc;

#[derive(Debug)]
pub struct UserActionLogService {
    dao: BaseRepository<UserActionLog>,
}

impl UserActionLogService {
    fn build(&self, agent_id:&str,user_id: impl AsRef<str>, remark: impl AsRef<str>, reason: impl AsRef<str>, operator_user: Option<impl AsRef<str>>) -> UserActionLog {
        let mut log = UserActionLog::default();
        log.user_id = as_ref_to_string(user_id);
        log.remark = Option::Some(as_ref_to_string(remark));
        log.agent_id= agent_id.to_string();
        log.reason = Option::Some(as_ref_to_string(reason));
        log.sync_statue = false;
        match operator_user {
            Some(user_id) => {
                log.operator_id = as_ref_to_string(user_id);
            }
            _ => {
                log.operator_id = "system".to_string();
            }
        }
        return log;
    }
    ///
    /// 强制下线
    pub async fn offline(&self,agent_id:&str, user_id: impl AsRef<str>, remark: impl AsRef<str>, reason: impl AsRef<str>, operator_user: Option<&str>) -> Result<(), AppError> {
        let mut action_log = self.build(agent_id,user_id, remark, reason, operator_user);
        action_log.action = UserActionType::Unmute;
        self.dao.insert(&action_log).await?;
        Ok(())
    }
    //禁言
    pub async fn mute(
        &self,
        agent_id: &str,
        user_id: &UserId,
        remark: &str,
        reason: &str,
        operator_user: Option<&str>,
    ) -> Result<()> {
        let client_service = ClientService::get();
        // 通过 user_id 查找用户，不存在则抛出业务错误
        let mut result = client_service
            .find_by_user_id(agent_id, user_id)
            .await?
            .ok_or_else(|| AppError::BizError("user.not_found".to_string()))?;
        // 构造操作日志
        let mut action_log = self.build(agent_id, user_id, remark, reason, operator_user);
        action_log.action = UserActionType::Mute;

        client_service.dao.up_property(&result.id,"message_status",true).await?;
        result.message_status=true;
        UserManager::get().sync_user(result).await?;
        // 插入操作记录日志
        self.dao.insert(&action_log).await?;
        Ok(())
    }

    pub async fn lock(
        &self,
        agent_id: &str,
        user_id: &UserId,
        remark: &str,
        reason: &str,
        operator_user: Option<&str>,
    ) -> Result<()> {
        let client_service = ClientService::get();
        // 通过 user_id 查找用户，不存在则抛出业务错误
        let mut result = client_service
            .find_by_user_id(agent_id, user_id)
            .await?
            .ok_or_else(|| AppError::BizError("user.not_found".to_string()))?;
        // 构造操作日志
        let mut action_log = self.build(agent_id, user_id, remark, reason, operator_user);
        action_log.action = UserActionType::Knockout;
        client_service.dao.up_property(&result.id,"lock",true).await?;
        result.lock=true;
        UserManager::get().sync_user(result).await?;
        // 插入操作记录日志
        self.dao.insert(&action_log).await?;
        Ok(())
    }
    //解禁
    pub async fn unmute(&self, agent_id:&str,user_id: &UserId, remark: &str, reason: &str, operator_user: Option<&str>) -> Result<()> {
        let client_service = ClientService::get();
        let client = client_service.find_by_user_id(agent_id, user_id).await?
            .ok_or_else(|| AppError::BizError("user.not_found".to_string()))?;
        //修改用户禁言状态
        //redis还需要同步更新
        client_service.enable_message(agent_id, user_id, false).await?;
        let mut action_log = self.build(agent_id,user_id, remark, reason, operator_user);
        action_log.action = UserActionType::Unmute;
        self.dao.insert(&action_log).await?;
        Ok(())
    }

    pub async fn ban(&self, agent_id:&str,user_id:&UserId, remark:&str, reason: &str, operator_user: Option<&str>) -> Result<()> {
        let service = ClientService::get();
        let client = service.find_by_user_id(agent_id,user_id).await?
            .ok_or_else(|| AppError::BizError("user.not_found".to_string()))?;
        //修改用户禁言状态
        //redis还需要同步更新
        service.dao.up_property(&client.id, "enable", true).await?;
        let mut action_log = self.build(agent_id,user_id, remark, reason, operator_user);
        action_log.action = UserActionType::Ban;
        self.dao.insert(&action_log).await?;
        Ok(())
    }
    pub async fn un_ban(&self, agent_id:&str,user_id: &UserId, remark: &str, reason: &str, operator_user: Option<&str>) -> Result<()> {
        let service = ClientService::get();
        let client = service.find_by_user_id(agent_id,user_id).await?
            .ok_or_else(|| AppError::BizError("user.not_found".to_string()))?;
        //修改用户禁言状态
        //redis还需要同步更新
        service.dao.up_property(&client.id, "enable", true).await?;
        let mut action_log = self.build(agent_id,user_id, remark, reason, operator_user);
        action_log.action = UserActionType::Unban;
        self.dao.insert(&action_log).await?;
        Ok(())
    }

    pub async fn login(&self, agent_id:&str,user_id: UserId, remark: &str, reason: &str, operator_user: Option<&str>) -> Result<()> {
        let mut action_log = self.build(agent_id,user_id, remark, reason, operator_user);
        action_log.action = UserActionType::Login;
        self.dao.insert(&action_log).await?;
        Ok(())
    }

    pub async fn warn(&self, agent_id:&str,user_id:UserId, remark: &str, reason: &str, operator_user: Option<&str>) -> Result<()> {
        let mut action_log = self.build(agent_id,user_id, remark, reason, operator_user);
        action_log.action = UserActionType::Warn;
        self.dao.insert(&action_log).await?;
        Ok(())
    }

    pub async fn block(&self, agent_id:&str,user_id: &str, remark: &str, reason: &str, operator_user: Option<&str>) -> Result<()> {
        let mut action_log = self.build(agent_id,user_id, remark, reason, operator_user);
        action_log.action = UserActionType::Block;
        self.dao.insert(&action_log).await?;
        Ok(())
    }

    pub async fn un_block(&self, agent_id:&str,user_id: UserId, remark: impl AsRef<str>, reason: impl AsRef<str>, operator_user: Option<String>) -> Result<()> {
        let mut action_log = self.build(agent_id,user_id, remark, reason, operator_user);
        action_log.action = UserActionType::Unblock;
        self.dao.insert(&action_log).await?;
        Ok(())
    }

    pub fn new(db: Database) -> Self {
        let collection = db.collection("mq_user_action_log");
        Self { dao: BaseRepository::new(db, collection.clone()) }
    }
    pub fn init(db: Database) {
        let instance = Self::new(db);
        INSTANCE.set(Arc::new(instance)).expect("INSTANCE already initialized");
    }

    /// 获取单例
    pub fn get() -> Arc<Self> {
        INSTANCE.get().expect("INSTANCE is not initialized").clone()
    }
    //强制下线
}
static INSTANCE: OnceCell<Arc<UserActionLogService>> = OnceCell::new();
