use common::repository_util::BaseRepository;
use mongodb::Database;
use once_cell::sync::OnceCell;
use std::sync::Arc;
use crate::protocol::friend::FriendEventMsg;

/// 好友事件服务
#[derive(Debug)]
pub struct FriendEventService {
    pub dao: BaseRepository<FriendEventMsg>,
}

impl FriendEventService {
    /// 创建服务实例
    pub fn new(db: Database) -> Self {
        let collection = db.collection("friend_event");
        Self { dao: BaseRepository::new(db, collection.clone()) }
    }

    /// 初始化全局单例
    pub fn init(db: Database) {
        let instance = Self::new(db);
        INSTANCE_FRIEND_EVENT.set(Arc::new(instance)).expect("INSTANCE_FRIEND_EVENT already initialized");
    }

    /// 获取服务单例
    pub fn get() -> Arc<Self> {
        INSTANCE_FRIEND_EVENT.get().expect("INSTANCE_FRIEND_EVENT is not initialized").clone()
    }
}

/// 全局单例：FriendEventService
static INSTANCE_FRIEND_EVENT: OnceCell<Arc<FriendEventService>> = OnceCell::new();
