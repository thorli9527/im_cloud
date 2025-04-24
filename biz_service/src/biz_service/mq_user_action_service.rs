use crate::biz_service::client_service::ClientService;
use crate::entitys::mq_user_action::{UserActionLog, UserActionType};
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
    fn build(&self, user_id: impl AsRef<str>, remark: impl AsRef<str>, reason: impl AsRef<str>, operator_user: Option<impl AsRef<str>>) -> UserActionLog {
        let mut log = UserActionLog::default();
        log.user_id = as_ref_to_string(user_id);
        log.remark = Option::Some(as_ref_to_string(remark));
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
    pub async fn offline(&self, user_id: impl AsRef<str>, remark: impl AsRef<str>, reason: impl AsRef<str>, operator_user: Option<&str>) -> Result<(), AppError> {
        let mut action_log = self.build(user_id, remark, reason, operator_user);
        action_log.action = UserActionType::Unmute;
        self.dao.insert(&action_log).await?;
        Ok(())
    }
    //禁言
    pub async fn mute(&self, user_id: impl AsRef<str>, remark: impl AsRef<str>, reason: impl AsRef<str>, operator_user: Option<&str>, time: Option<i64>) -> Result<(), AppError> {
        let service = ClientService::get();
        let result = service.find_by_user_id(user_id.as_ref()).await?;
        let mut action_log = self.build(&user_id, remark, reason, operator_user);
        action_log.action = UserActionType::Mute;
        match time {
            Some(end_time) => {
                action_log.expired_at = time;
                //修改数据库禁言时间
                //redis还需要同步更新
                service.dao.up_property(result.id, "message_expired_at", end_time).await?
            }
            (_) => {}
        }
        self.dao.insert(&action_log).await?;
        Ok(())
    }
    //解禁
    pub async fn unmute(&self, user_id: impl AsRef<str>, remark: impl AsRef<str>, reason: impl AsRef<str>, operator_user: Option<&str>) -> Result<(), AppError> {
        let service = ClientService::get();
        let client = service.find_by_user_id(user_id.as_ref()).await?;
        //修改用户禁言状态
        //redis还需要同步更新
        service.enable_message(client.id).await?;
        let mut action_log = self.build(user_id, remark, reason, operator_user);
        action_log.action = UserActionType::Unmute;
        self.dao.insert(&action_log).await?;
        Ok(())
    }

    pub async fn ban(&self, user_id: impl AsRef<str>, remark: impl AsRef<str>, reason: impl AsRef<str>, operator_user: Option<&str>) -> Result<(), AppError> {
        let service = ClientService::get();
        let client = service.find_by_user_id(user_id.as_ref()).await?;
        //修改用户禁言状态
        //redis还需要同步更新
        service.dao.up_property(&client.id, "enable", true).await?;
        let mut action_log = self.build(user_id, remark, reason, operator_user);
        action_log.action = UserActionType::Ban;
        self.dao.insert(&action_log).await?;
        Ok(())
    }
    pub async fn un_ban(&self, user_id: impl AsRef<str>, remark: impl AsRef<str>, reason: impl AsRef<str>, operator_user: Option<&str>) -> Result<(), AppError> {
        let service = ClientService::get();
        let client = service.find_by_user_id(user_id.as_ref()).await?;
        //修改用户禁言状态
        //redis还需要同步更新
        service.dao.up_property(&client.id, "enable", true).await?;
        let mut action_log = self.build(user_id, remark, reason, operator_user);
        action_log.action = UserActionType::Unban;
        self.dao.insert(&action_log).await?;
        Ok(())
    }

    pub async fn login(&self, user_id: impl AsRef<str>, remark: impl AsRef<str>, reason: impl AsRef<str>, operator_user: Option<&str>) -> Result<(), AppError> {
        let mut action_log = self.build(user_id, remark, reason, operator_user);
        action_log.action = UserActionType::Login;
        self.dao.insert(&action_log).await?;
        Ok(())
    }

    pub async fn warn(&self, user_id: impl AsRef<str>, remark: impl AsRef<str>, reason: impl AsRef<str>, operator_user: Option<&str>) -> Result<(), AppError> {
        let mut action_log = self.build(user_id, remark, reason, operator_user);
        action_log.action = UserActionType::Warn;
        self.dao.insert(&action_log).await?;
        Ok(())
    }

    pub async fn block(&self, user_id: impl AsRef<str>, remark: impl AsRef<str>, reason: impl AsRef<str>, operator_user: Option<&str>) -> Result<(), AppError> {
        let mut action_log = self.build(user_id, remark, reason, operator_user);
        action_log.action = UserActionType::Block;
        self.dao.insert(&action_log).await?;
        Ok(())
    }

    pub async fn un_block(&self, user_id: impl AsRef<str>, remark: impl AsRef<str>, reason: impl AsRef<str>, operator_user: Option<String>) -> Result<(), AppError> {
        let mut action_log = self.build(user_id, remark, reason, operator_user);
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
