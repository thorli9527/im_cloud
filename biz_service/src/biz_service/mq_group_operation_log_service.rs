use crate::entitys::mq_group_operation_log::{GroupOperationLog, GroupOperationType};
use anyhow::Result;
use common::errors::AppError;
use common::repository_util::{BaseRepository, Repository};
use common::util::common_utils::as_ref_to_string;
use mongodb::Database;
use once_cell::sync::OnceCell;
use std::sync::Arc;
#[derive(Debug)]
pub struct GroupOperationLogService {
    pub dao: BaseRepository<GroupOperationLog>,
}

impl GroupOperationLogService {
    pub fn new(db: Database) -> Self {
        let collection = db.collection("group_operation_log");
        Self { dao: BaseRepository::new(db, collection.clone()) }
    }

    fn build(&self,agent_id:&str, group_id: impl AsRef<str>, user_id: impl AsRef<str>, operator_id: Option<String>) -> GroupOperationLog {
        let mut log = GroupOperationLog::default();
        log.group_id = as_ref_to_string(group_id);
        log.agent_id = agent_id.to_string();
        log.target_id = as_ref_to_string(user_id);
        match operator_id {
            Some(operator) => {
                log.operator_id = operator;
            }
            _ => {
                log.operator_id = "system".to_string();
            }
        }
        log.sync_statue = false;
        log
    }
    pub async fn add_log(&self,agent_id:&str, group_id: impl AsRef<str>, user_id: impl AsRef<str>, operator_user: Option<String>, action: GroupOperationType) -> Result<(), AppError> {
        let mut action_log = self.build(agent_id,group_id, user_id, operator_user);
        action_log.action = action;
        match action {
            GroupOperationType::Mute => {
               return Err(AppError::BizError("please.call.method.add_log_expired_at".to_string()))
            }
            _ => {}
        }
        self.dao.insert(&action_log).await?;
        Ok(())
    }

    pub fn init(db: Database) {
        let instance = Self::new(db);
        INSTANCE.set(Arc::new(instance)).expect("INSTANCE already initialized");
    }

    /// 获取单例
    pub fn get() -> Arc<Self> {
        INSTANCE.get().expect("INSTANCE is not initialized").clone()
    }
}
static INSTANCE: OnceCell<Arc<GroupOperationLogService>> = OnceCell::new();
