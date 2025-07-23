use crate::biz_service::group_member_service::GroupMemberService;
use crate::biz_service::user_service::UserService;
use crate::protocol::common::{GroupEntity, GroupMemberEntity, GroupRoleType};
use anyhow::{Result, anyhow};
use common::UserId;
use common::errors::AppError;
use common::redis::redis_pool::RedisPoolTools;
use common::repository_util::{BaseRepository, Repository};
use common::util::common_utils::build_uuid;
use common::util::date_util::now;
use deadpool_redis::redis::AsyncCommands;
use mongodb::bson::doc;
use mongodb::{Collection, Database};
use once_cell::sync::OnceCell;
use serde_json::json;
use std::sync::Arc;

#[derive(Debug)]
pub struct GroupService {
    pub dao: BaseRepository<GroupEntity>,
    pub db: Database,
}

impl GroupService {
    pub fn new(db: Database) -> Self {
        let collection = db.collection("group_info");
        Self { dao: BaseRepository::new(db.clone(), collection.clone()), db }
    }

    pub async fn find_by_group_id(
        &self,
        group_id: impl AsRef<str>,
    ) -> Result<GroupEntity, AppError> {
        let result = self.dao.find_by_id(group_id.as_ref()).await?;
        match result {
            Some(group) => Ok(group),
            None => Err(AppError::BizError("Group not found".to_string())),
        }
    }

    pub async fn create_group(&self, group: &GroupEntity, ids: &Vec<String>) -> anyhow::Result<()> {
        let old_info = self
            .dao
            .find_one(doc! {"name": group.clone().name,"owner_id": group.clone().owner_id})
            .await?;
        if old_info.is_some() {
            return Err(anyhow!("group.already.exists"));
        }
        // 0. 获取 owner 用户
        let user_service = UserService::get();
        let user_info = user_service.dao.find_by_id(group.owner_id.as_str()).await?;
        if user_info.is_none() {
            return Err(anyhow!("user.not.found"));
        }
        let user = user_info.unwrap();

        // 1. 准备事务资源
        let database = self.db.clone();
        let client = database.client();
        let mut session = client.start_session().await?;
        session.start_transaction().await?;

        let group_coll: Collection<GroupEntity> = database.collection("group_info");
        let group_member_coll: Collection<GroupMemberEntity> = database.collection("group_member");

        let now = now() as u64;
        let group_id = group.id.to_string();

        let result = async {
            // 2. 插入群组
            group_coll.insert_one(group).session(&mut session).await?;
            // 3. 插入群主为成员
            let owner = GroupMemberEntity {
                id: build_uuid(),
                group_id: group_id.clone(),
                uid: group.owner_id.clone(),
                role: GroupRoleType::Owner as i32,
                is_muted: false,
                alias: user.user_name.clone(),
                create_time: now,
                update_time: now,
                avatar: "".to_string(),
            };
            group_member_coll.insert_one(owner).session(&mut session).await?;

            // 4. 插入其余初始成员
            for user_id in ids {
                if user_id == &group.owner_id {
                    continue; // 跳过群主
                }
                let member = GroupMemberEntity {
                    id: build_uuid(),
                    group_id: group_id.clone(),
                    uid: user_id.clone(),
                    role: GroupRoleType::Member as i32,
                    is_muted: false,
                    alias: "".to_string(),
                    create_time: now,
                    update_time: now,
                    avatar: "".to_string(),
                };
                group_member_coll.insert_one(member).session(&mut session).await?;
                // ============ Step 6: 写入 Redis 缓存 ============= //
                let redis = RedisPoolTools::get();
                let mut conn = redis.get().await?;

                // 缓存 key
                let member_key = format!("group:members:{}", group_id);
                let admin_key = format!("group:admin:{}", group_id);
                let group_info_key = format!("group:info:{}", group_id);

                // 添加群主
                conn.sadd::<_, _, ()>(&member_key, &group.owner_id).await?;
                conn.sadd::<_, _, ()>(&admin_key, &group.owner_id).await?;

                // 添加初始成员
                if !ids.is_empty() {
                    let other_members: Vec<String> =
                        ids.iter().filter(|u| *u != &group.owner_id).cloned().collect();
                    if !other_members.is_empty() {
                        conn.sadd::<_, _, ()>(&member_key, other_members).await?;
                    }
                }

                // ✨ 新增：序列化群组信息并缓存
                let group_json = serde_json::to_string(group)?;
                conn.set_ex::<_, _, ()>(&group_info_key, group_json, 7 * 24 * 3600).await?; // 7 天过期

                tracing::info!("✅ Redis 缓存初始化成功: group_id={}", group_id);
            }
            Ok::<(), anyhow::Error>(())
        }
        .await;

        // 5. 提交或回滚事务
        match result {
            Ok(_) => session.commit_transaction().await?,
            Err(e) => {
                session.abort_transaction().await?;
                return Err(e);
            }
        }

        Ok(())
    }

    /// 解散群组（MongoDB + Redis 全清理）
    pub async fn dismiss_group(&self, group_id: &str, operator_id: &str) -> Result<(), AppError> {
        let group_member_service = GroupMemberService::get();

        // Step 1: 开启 MongoDB 事务
        let db = self.db.clone();
        let client = db.client();
        let mut session = client.start_session().await?;
        session.start_transaction().await?;

        let group_coll = self.dao.collection.clone();
        let member_coll = group_member_service.dao.collection.clone();

        let txn_result = async {
            // Step 2: 删除所有成员记录
            member_coll.delete_many(doc! { "group_id": group_id }).session(&mut session).await?;

            // Step 3: 删除群组信息
            group_coll.delete_one(doc! { "_id": group_id }).session(&mut session).await?;

            Ok::<(), AppError>(())
        }
        .await;

        match txn_result {
            Ok(_) => {
                session.commit_transaction().await?;

                // Step 4: 清理 Redis 缓存
                let mut conn = RedisPoolTools::get().get().await?;

                let keys_to_delete = vec![
                    format!("group:members:{}", group_id),
                    format!("group:admin:{}", group_id),
                    format!("group:info:{}", group_id),
                ];

                conn.del::<_, ()>(keys_to_delete).await?;

                tracing::info!("✅ 群组 {} 删除成功，缓存清理完毕", group_id);
                Ok(())
            }
            Err(e) => {
                session.abort_transaction().await?;
                tracing::error!("❌ 解散群组失败：group_id={}，err={}", group_id, e);
                Err(e)
            }
        }
    }

    pub async fn update_group_name(&self, group_id: &str, new_name: &str) -> Result<(), AppError> {
        // Step 1: 更新 MongoDB
        let filter = doc! { "group_id": &group_id };
        let update_time = now();
        let update = doc! {
            "$set": {
                "name": &new_name,
                "update_time": update_time,
            }
        };
        self.dao.update(filter, update).await?;

        // Step 2: 更新 Redis 缓存
        let mut conn = RedisPoolTools::get().get().await?;
        let redis_key = format!("group:info:{}", group_id);

        // 如果你希望保留旧值，可以先查旧缓存，但此处直接更新
        let group_info = json!({
            "group_id": group_id,
            "name": new_name,
            "update_time": update_time,
        });

        conn.set::<_, _, ()>(&redis_key, serde_json::to_string(&group_info)?).await?;

        tracing::info!("✅ 群组名称已更新并同步 Redis: group_id={}, name={}", group_id, new_name);

        Ok(())
    }
    /// 转让群组（变更 creator_id）

    pub async fn transfer_ownership(&self, group_id: &str, new_owner_id: &UserId) -> Result<()> {
        // Step 1: 获取原群信息
        let group = self.dao.find_one(doc! { "_id": &group_id }).await?;
        if group.is_none() {
            return Err(anyhow!("群组不存在"));
        }
        let group = group.unwrap();
        let old_owner_id = group.id.clone();

        // Step 2: 校验新群主是否是群成员
        let member_service = GroupMemberService::get();
        member_service
            .find_by_group_id_and_uid(group_id, new_owner_id)
            .await
            .map_err(|_| AppError::BizError("new.owner.not.member".into()))?;

        // Step 3: 更新群组信息中的群主字段
        let update = doc! {
            "$set": {
                "owner_id": &new_owner_id,
                "update_time": now(),
            }
        };
        self.dao.update(doc! { "_id": &group_id }, update).await?;

        // Step 4: 修改群成员角色（旧群主 → Member， 新群主 → Owner）
        member_service.change_member_role(&group_id, &old_owner_id, &GroupRoleType::Member).await?;

        member_service.change_member_role(&group_id, &new_owner_id, &GroupRoleType::Owner).await?;

        // Step 5: 更新 Redis 中管理员集合
        let redis = RedisPoolTools::get();
        let mut conn = redis.get().await?;
        let admin_key = format!("group:admin:{}", group_id);

        conn.srem::<_, _, ()>(&admin_key, &old_owner_id).await?;
        conn.sadd::<_, _, ()>(&admin_key, &new_owner_id).await?;

        // Step 6: 可选：刷新 Redis 中的群组信息缓存（如果存在）
        let info_key = format!("group:info:{}", group_id);
        let group_info = json!({
            "group_id": group_id,
            "owner_id": new_owner_id,
            "name": group.name,
            "update_time": now(),
        });
        conn.set::<_, _, ()>(&info_key, serde_json::to_string(&group_info)?).await?;

        tracing::info!(
            "✅ 群主转让成功: group_id={} from={} → to={}",
            group_id,
            old_owner_id,
            new_owner_id
        );
        Ok(())
    }

    pub async fn query_admin_member_by_group_id(&self, group_id: &str) -> Result<Vec<String>> {
        let redis = RedisPoolTools::get();
        let mut conn = redis.get().await?;
        let admin_key = format!("group:admin:{}", group_id);

        // 从 Redis 获取管理员列表
        let admins: Vec<String> = conn.smembers(&admin_key).await?;

        if admins.is_empty() {
            return Err(anyhow!("群组不存在"));
        }
        Ok(admins)
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
static INSTANCE: OnceCell<Arc<GroupService>> = OnceCell::new();
