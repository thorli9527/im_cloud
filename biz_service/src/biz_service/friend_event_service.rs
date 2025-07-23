use crate::entitys::friend::FriendEntity;
use crate::protocol::msg::friend::{EventStatus, FriendEventMsg, FriendEventType};
use anyhow::Result;
use bson::doc;
use bson::oid::ObjectId;
use common::repository_util::{BaseRepository, Repository};
use common::util::common_utils::build_snow_id;
use common::util::date_util::now;
use common::UserId;
use log::warn;
use mongodb::{Collection, Database};
use once_cell::sync::OnceCell;
use std::sync::Arc;

/// 好友事件服务
#[derive(Debug)]
pub struct FriendEventService {
    pub dao: BaseRepository<FriendEventMsg>,
    pub db: Database,
}

impl FriendEventService {
    /// 创建服务实例
    pub fn new(db: Database) -> Self {
        let collection = db.collection("friend_event");
        Self { db: db.clone(), dao: BaseRepository::new(db, collection.clone()) }
    }

    /// 申请好友
    pub async fn apply_friend(&self, event: &FriendEventMsg) -> Result<()> {
        // 存储好友申请事件
        self.dao.insert(event).await?;
        Ok(())
    }
    /// 受理好友申请
    pub async fn accept_friend(
        &self,
        event_id: &String,
        uid: &UserId,
        a_name: &str,
        remark: Option<&str>,
    ) -> Result<()> {
        let info = self.dao.find_by_id(event_id).await?;
        if info.is_none() {
            return Err(anyhow::anyhow!("申请记录不存在"));
        }
        let event = info.unwrap();
        if event.event_type != FriendEventType::FriendRequest as i32 {
            warn!("事件类型错误: event_id:{} type :{}", event_id, event.event_type);
            return Err(anyhow::anyhow!("事件类型错误"));
        }

        if event.to_uid != uid.clone() {
            warn!("事件错误: event_id:{} to_uid:{}", event_id, uid);
            return Err(anyhow::anyhow!("事件错误，非当前用户的好友申请"));
        }

        // 1. 更新好友申请事件状态为已受理

        let database = self.db.clone();
        let client = database.client();
        let friend_event_coll: Collection<FriendEventMsg> = database.collection("friend_event");
        let user_friend_coll: Collection<FriendEntity> = database.collection("user_friend");

        let mut session = client.start_session().await?;
        let now = now() as u64;
        session.start_transaction().await?;
        let result = async {
            let from_entity = FriendEntity {
                id: ObjectId::new().to_hex(),
                uid: event.from_uid.clone(),
                friend_id: event.to_uid.clone(),
                nickname: event.to_a_name.clone(),
                remark: event.to_remark.clone(),
                is_blocked: false,
                source_type: event.source_type(),
                created_at: now,
            };
            user_friend_coll.insert_one(from_entity).session(&mut session).await?;
            let to_entity = FriendEntity {
                id: ObjectId::new().to_hex(),
                uid: event.to_uid.clone(),
                friend_id: event.from_uid.clone(),
                nickname: event.from_a_name.clone(),
                remark: event.from_remark.clone(),
                is_blocked: false,
                source_type: event.source_type(),
                created_at: now,
            };
            user_friend_coll.insert_one(to_entity).session(&mut session).await?;

            friend_event_coll
                .update_one(
                    doc! { "_id": event_id },
                    doc! {
                            "$set": {
                                "event_type": FriendEventType::FriendAccept as i32 ,
                                "status": EventStatus::Done as i32,
                                "to_a_name":a_name.to_string(),
                                "to_remark": remark.clone()
                            }
                    },
                )
                .session(&mut session)
                .await?;
            Ok::<(), anyhow::Error>(())
        }
        .await;
        match result {
            Ok(_) => {
                session.commit_transaction().await?;
            }
            Err(e) => {
                log::error!("{}", e);
                session.abort_transaction().await?;
            }
        }

        Ok(())
    }

    /// 删除好友关系
    pub async fn remove_friend(&self, uid: &UserId, friend_id: &UserId) -> Result<()> {
        let database = self.db.clone();
        let client = database.client();
        let user_friend_coll: Collection<FriendEntity> = database.collection("user_friend");
        let friend_event_coll: Collection<FriendEventMsg> = database.collection("friend_event");

        let mut session = client.start_session().await?;
        session.start_transaction().await?;

        let now = now() as u64;
        let result = async {
            // 删除双方好友关系
            user_friend_coll
                .delete_many(doc! {
                    "$or": [
                        { "uid": uid, "friend_id": friend_id },
                        { "uid": friend_id, "friend_id": uid },
                    ]
                })
                .session(&mut session)
                .await?;

            // 可选：记录一条删除事件日志（用于审计或同步）
            let event = FriendEventMsg {
                message_id: build_snow_id(),
                from_uid: uid.clone(),
                to_uid: friend_id.clone(),
                event_type: FriendEventType::FriendRemove as i32,
                message: "".to_string(),
                status: EventStatus::Done as i32,
                created_at: now,
                updated_at: now,
                source_type: 0,
                from_a_name: "".to_string(),
                from_remark: None,
                to_a_name: "".to_string(),
                to_remark: None,
            };
            friend_event_coll.insert_one(event).session(&mut session).await?;

            Ok::<(), anyhow::Error>(())
        }
        .await;

        match result {
            Ok(_) => session.commit_transaction().await?,
            Err(e) => {
                session.abort_transaction().await?;
                return Err(e);
            }
        }

        Ok(())
    }

    /// 初始化全局单例
    pub fn init(db: Database) {
        let instance = Self::new(db);
        INSTANCE_FRIEND_EVENT
            .set(Arc::new(instance))
            .expect("INSTANCE_FRIEND_EVENT already initialized");
    }

    /// 获取服务单例
    pub fn get() -> Arc<Self> {
        INSTANCE_FRIEND_EVENT.get().expect("INSTANCE_FRIEND_EVENT is not initialized").clone()
    }
}

/// 全局单例：FriendEventService
static INSTANCE_FRIEND_EVENT: OnceCell<Arc<FriendEventService>> = OnceCell::new();
