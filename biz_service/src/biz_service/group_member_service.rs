use crate::protocol::common::{GroupMemberEntity, GroupRoleType};
use anyhow::{Result, anyhow};
use common::UserId;
use common::errors::AppError;
use common::redis::redis_pool::RedisPoolTools;
use common::repository_util::{BaseRepository, OrderType, PageResult, Repository};
use common::util::date_util::now;
use deadpool_redis::redis;
use deadpool_redis::redis::AsyncCommands;
use mongodb::{Collection, Database};
use mongodb::bson::doc;
use once_cell::sync::OnceCell;
use std::sync::Arc;
use bson::Document;

#[derive(Debug)]
pub struct GroupMemberService {
    pub dao: BaseRepository<GroupMemberEntity>,
    pub db: Database,
}

impl GroupMemberService {
    pub fn new(db: Database) -> Self {
        let collection = db.collection("group_member");
        Self {
            dao: BaseRepository::new(db.clone(), collection.clone()),
            db: db,
        }
    }

    pub fn init(db: Database) {
        let instance = Self::new(db);
        INSTANCE
            .set(Arc::new(instance))
            .expect("INSTANCE already initialized");
    }

    pub fn key_group_members(group_id: &str) -> String {
        format!("group:member:{}", group_id)
    }
    pub async fn add_members(&self, members: &[GroupMemberEntity]) -> Result<()> {
        if members.is_empty() {
            return Ok(()); // 空输入跳过
        }

        let db = self.dao.db.clone();
        let client = db.client();
        let mut session = client.start_session().await?;
        session.start_transaction().await?;

        let coll: Collection<GroupMemberEntity> = self.db.collection("group_member");
        let mut conn = RedisPoolTools::get().get().await?;
        let mut added_count = 0;

        let result = async {
            for member in members {
                let group_id = &member.group_id;
                let user_id = &member.uid;
                // 检查是否已存在（MongoDB）
                let exists = coll
                    .find_one(doc! {
                        "group_id": group_id,
                        "user_id": user_id
                    })
                    .await?;

                if exists.is_some() {
                    tracing::warn!(
                        "⚠️ 群成员已存在，跳过添加: group_id={}, user_id={}",
                        group_id,
                        user_id
                    );
                    continue;
                }

                // 插入 MongoDB（事务中）
                coll.insert_one(member).session(&mut session).await?;

                added_count += 1;
            }

            Ok::<(), anyhow::Error>(())
        }
        .await;

        match result {
            Ok(_) => {
                session.commit_transaction().await?;

                // MongoDB 提交成功后 → 写入 Redis 缓存
                for member in members {
                    let group_id = &member.group_id;
                    let user_id = &member.uid;
                    let role =
                        GroupRoleType::from_i32(member.role).unwrap_or(GroupRoleType::Member);

                    let member_set_key = Self::key_group_members(group_id);
                    let admin_key = format!("group:admin:{}", group_id);
                    let detail_key = group_member_key(group_id, user_id);

                    // 写入成员集合
                    conn.sadd::<_, _, ()>(&member_set_key, user_id).await?;

                    // 如果是 Admin 或 Owner，加到管理员集合
                    if matches!(role, GroupRoleType::Admin | GroupRoleType::Owner) {
                        conn.sadd::<_, _, ()>(&admin_key, user_id).await?;
                    }

                    // 写入成员详情缓存，TTL=3600
                    let json = serde_json::to_string(member)?;
                    conn.set_ex::<_, _, ()>(&detail_key, json, 3600).await?;
                }

                tracing::info!(
                    "✅ 批量添加群成员成功：新增 {} 条，TTL = 3600s",
                    added_count
                );
                Ok(())
            }
            Err(e) => {
                session.abort_transaction().await?;
                tracing::error!("❌ 添加群成员失败，事务已回滚: {}", e);
                Err(e)
            }
        }
    }

    pub async fn remove(&self, group_id: &str, user_id: &UserId) -> Result<()> {
        // Step 1: 验证成员角色
        let member = self.find_by_group_id_and_uid(group_id, user_id).await?;
        let role = GroupRoleType::from_i32(member.role)
            .ok_or_else(|| anyhow!("group.member.role.invalid"))?;

        if role == GroupRoleType::Owner {
            return Err(anyhow!("group.owner.cannot.be.removed"));
        }

        // Step 2: 删除 MongoDB 成员记录
        let filter = doc! {
            "group_id": group_id,
            "user_id": user_id,
        };
        self.dao.delete(filter).await?;

        // Step 3: 清理 Redis 缓存
        let mut conn = RedisPoolTools::get().get().await?;
        let member_key = format!("group:members:{}", group_id);
        let admin_key = format!("group:admin:{}", group_id);
        let detail_key = group_member_key(group_id, user_id);

        // 批量执行 Redis 删除
        let (removed_from_set, removed_from_admin, deleted_detail): (i32, i32, i32) = redis::pipe()
            .atomic()
            .cmd("SREM")
            .arg(&member_key)
            .arg(user_id)
            .cmd("SREM")
            .arg(&admin_key)
            .arg(user_id)
            .cmd("DEL")
            .arg(&detail_key)
            .query_async(&mut conn)
            .await
            .unwrap_or((0, 0, 0));

        tracing::info!(
            "🗑️ 成员移除成功: group_id={}, user_id={}, redis[srem:{}+{} del:{}]",
            group_id,
            user_id,
            removed_from_set,
            removed_from_admin,
            deleted_detail
        );

        Ok(())
    }

    /// 从缓存优先读取分页成员列表，否则回源 MongoDB
    pub async fn find_by_group_id(
        &self,
        group_id: &str,
        page_size: i64,
        order_type: Option<OrderType>,
        sort_field: &str,
    ) -> Result<PageResult<GroupMemberEntity>> {
        let set_key = Self::key_group_members(group_id); // Redis Set key
        let hash_key = group_member_list_key(group_id); // Redis Hash: group:member:list:{group_id}
        let mut conn = RedisPoolTools::get().get().await?;

        // Step 1: Redis 获取所有成员 uid（用于分页）
        let all_uids: Vec<UserId> = conn.smembers(&set_key).await.unwrap_or_default();

        if !all_uids.is_empty() {
            let mut entities = Vec::with_capacity(all_uids.len());
            for uid in &all_uids {
                let redis_key = group_member_key(group_id, uid);
                if let Ok(Some(json)) = conn.get::<_, Option<String>>(&redis_key).await {
                    if let Ok(entity) = serde_json::from_str::<GroupMemberEntity>(&json) {
                        entities.push(entity);
                    }
                }
            }

            // Step 2: 排序 + 分页
            if !entities.is_empty() {
                // 可选排序（默认按 create_time 升序）
                let mut entities = entities;
                entities.sort_by(|a, b| match sort_field {
                    "create_time" => {
                        if order_type == Some(OrderType::Desc) {
                            b.create_time.cmp(&a.create_time)
                        } else {
                            a.create_time.cmp(&b.create_time)
                        }
                    }
                    _ => a.uid.cmp(&b.uid), // 默认用 uid 排序
                });

                let has_more = entities.len() as i64 > page_size;
                if has_more {
                    entities.truncate(page_size as usize);
                }

                return Ok(PageResult {
                    items: entities,
                    has_next: has_more,
                    has_prev: false,
                });
            }
        }

        // Step 3: Redis 未命中 → MongoDB 查询 + 回写缓存
        let filter = doc! { "group_id": group_id };
        let page = self
            .dao
            .query_by_page(filter, page_size, order_type, sort_field)
            .await?;

        if !page.items.is_empty() {
            // Step 4: 写入 Redis（Set + Hash）
            let uids: Vec<UserId> = page.items.iter().map(|m| m.uid.clone()).collect();
            conn.sadd::<_, _, ()>(&set_key, &uids).await?;
            for m in &page.items {
                let json = serde_json::to_string(m)?;
                let detail_key = group_member_key(group_id, &m.uid);
                conn.set_ex::<_, _, ()>(&detail_key, json, 3600).await?;
            }
        }

        Ok(page)
    }

    /// 查询某个群组下所有成员（优先使用 Redis，未命中回源 MongoDB）
    /// 返回 Vec<GroupMemberEntity>
    pub async fn get_all_members_by_group_id(
        &self,
        group_id: &str,
    ) -> Result<Vec<GroupMemberEntity>> {
        let set_key = Self::key_group_members(group_id);
        let mut conn = RedisPoolTools::get().get().await?;

        // Step 1: 从 Redis Set 获取成员 uid 列表
        let uids: Vec<UserId> = conn.smembers(&set_key).await.unwrap_or_default();

        if !uids.is_empty() {
            let mut members = Vec::with_capacity(uids.len());
            for uid in &uids {
                let detail_key = group_member_key(group_id, uid);
                if let Ok(Some(json)) = conn.get::<_, Option<String>>(&detail_key).await {
                    if let Ok(entity) = serde_json::from_str::<GroupMemberEntity>(&json) {
                        members.push(entity);
                    }
                }
            }

            if !members.is_empty() {
                tracing::info!(
                    "✅ 从 Redis 加载群成员成功: group_id={}, count={}",
                    group_id,
                    members.len()
                );
                return Ok(members);
            }
        }

        // Step 2: Redis 未命中 → MongoDB 查询
        let filter = doc! { "group_id": group_id };
        let results = self.dao.query(filter).await?;

        // Step 3: 回写缓存（Set + 详情）
        if !results.is_empty() {
            let uids: Vec<UserId> = results.iter().map(|m| m.uid.clone()).collect();
            let _: () = conn.sadd(&set_key, &uids).await?;

            for member in &results {
                let detail_key = group_member_key(group_id, &member.uid);
                let json = serde_json::to_string(member)?;
                let _: () = conn.set_ex(detail_key, json, 3600).await?;
            }

            tracing::info!(
                "📦 MongoDB加载并缓存群成员: group_id={}, count={}",
                group_id,
                results.len()
            );
        }

        Ok(results)
    }

    /// 删除某个群组下所有成员（MongoDB + Redis）
    /// - MongoDB 批量删除群成员
    /// - Redis 清除成员 UID 集合及详情缓存
    pub async fn delete_by_group_id(&self, group_id:&str) -> Result<()> {

        // Step 1: 删除 MongoDB 中该群的所有成员记录
        let filter = doc! { "group_id":group_id };
        self.dao.delete(filter).await?;

        // Step 2: 清除 Redis 缓存（包括 Set 和 Hash）
        let mut conn = RedisPoolTools::get().get().await?;
        let member_set_key = format!("group:members:{}", group_id);
        let admin_set_key = format!("group:admin:{}", group_id);

        // 获取成员 UID 列表（用于逐个删除详情）
        let uids: Vec<UserId> = conn.smembers(&member_set_key).await.unwrap_or_default();

        // 删除详情缓存 key：group:member:{group_id}:{uid}
        if !uids.is_empty() {
            let detail_keys: Vec<String> = uids
                .iter()
                .map(|uid| group_member_key(group_id, uid))
                .collect();
            let _: () = conn.del(detail_keys).await?;
        }

        // 删除成员 UID 列表 和 管理员集合
        let _: () = conn.del(&[member_set_key, admin_set_key]).await?;

        tracing::info!("🧹 群组 {} 成员信息已删除（MongoDB + Redis）", group_id);

        Ok(())
    }

    /// 查询群主成员信息（先查 Redis，再查 MongoDB）
    /// Redis Key: group:owner:{group_id}
    pub async fn find_owner(&self, group_id: &str) -> Result<GroupMemberEntity> {
        let redis_key = format!("group:owner:{}", group_id);
        let mut conn = RedisPoolTools::get().get().await?;

        // Step 1: 先查 Redis 缓存
        if let Ok(Some(json)) = conn.get::<_, Option<String>>(&redis_key).await {
            if let Ok(entity) = serde_json::from_str::<GroupMemberEntity>(&json) {
                return Ok(entity);
            } else {
                tracing::warn!("❗Redis 命中但反序列化失败: {}", redis_key);
            }
        }

        // Step 2: 查 MongoDB
        let filter = doc! {
            "group_id": group_id,
            "role": GroupRoleType::Owner as i32
        };

        match self.dao.find_one(filter).await? {
            Some(owner) => {
                // Step 3: 写入缓存（默认 1 小时）
                let json = serde_json::to_string(&owner).unwrap_or_default();
                let _: () = conn
                    .set_ex(&redis_key, json, 3600)
                    .await
                    .unwrap_or_default();

                Ok(owner)
            }
            None => Err(anyhow!("group.owner.notfound")),
        }
    }

    pub async fn change_member_role(
        &self,
        group_id: &str,
        member_id: &UserId,
        new_role: &GroupRoleType,
    ) -> Result<()> {
        // Step 1: 查询原角色
        let existing = self.find_by_group_id_and_uid(group_id, member_id).await?;
        let old_role = GroupRoleType::from_i32(existing.role)
            .ok_or_else(|| AppError::BizError("group.member.role.invalid".to_string()))?;

        // Step 2: 拒绝任何涉及 Owner 的变更
        if old_role == GroupRoleType::Owner || new_role == &GroupRoleType::Owner {
            return Err(anyhow!("group.owner.role.cannot.change"));
        }

        // Step 3: 无需变更
        if old_role == *new_role {
            tracing::debug!(
                "ℹ️ 群成员角色无变更，跳过处理: group_id={}, user_id={}, role={:?}",
                group_id,
                member_id,
                new_role
            );
            return Ok(());
        }

        // Step 4: 更新 MongoDB
        let filter = doc! {
            "group_id": group_id,
            "user_id": member_id,
        };
        let update = doc! {
            "$set": {
                "role": *new_role as i32,
                "update_time": now(),
            }
        };
        self.dao.update(filter, update).await?;

        // Step 5: 更新 Redis 管理员集合
        let mut conn = RedisPoolTools::get().get().await?;
        let admin_key = format!("group:admin:{}", group_id);
        let user_id_str = member_id.to_string(); // 显式转换

        match (old_role, new_role) {
            (GroupRoleType::Member, GroupRoleType::Admin) => {
                conn.sadd::<_, _, ()>(&admin_key, &user_id_str).await?;
            }
            (GroupRoleType::Admin, GroupRoleType::Member) => {
                conn.srem::<_, _, ()>(&admin_key, &user_id_str).await?;
            }
            _ => {
                // 其他（如 Admin → Admin），不处理
            }
        }

        tracing::info!(
            "✅ 成员角色更新成功: group_id={}, user_id={}, {:?} → {:?}",
            group_id,
            member_id,
            old_role,
            new_role
        );

        Ok(())
    }

    pub async fn find_by_group_id_and_uid(
        &self,
        group_id: &str,
        uid: &UserId,
    ) -> Result<GroupMemberEntity> {
        let redis_key = group_member_key(group_id, uid);
        let mut conn = RedisPoolTools::get().get().await.map_err(|e| {
            tracing::error!("Redis连接失败: {}", e);
            AppError::Internal("redis.connect.fail".into())
        })?;

        if let Ok(Some(json_str)) = conn.get::<_, Option<String>>(&redis_key).await {
            if let Ok(entity) = serde_json::from_str::<GroupMemberEntity>(&json_str) {
                return Ok(entity);
            } else {
                tracing::warn!(
                    "❗Redis命中但反序列化失败 group_id={} uid={}",
                    group_id,
                    uid
                );
            }
        }

        let filter = doc! {"group_id": group_id, "user_id": uid};
        match self.dao.find_one(filter).await? {
            Some(entity) => {
                let cache_str = serde_json::to_string(&entity).unwrap_or_default();
                let _: () = conn
                    .set_ex(&redis_key, cache_str, 3600)
                    .await
                    .unwrap_or_default();
                Ok(entity)
            }
            None => {
                tracing::warn!("❌ MongoDB 未找到群成员 group_id={} uid={}", group_id, uid);
                Err(anyhow!("group.member.notfound"))
            }
        }
    }

    pub fn get() -> Arc<Self> {
        INSTANCE.get().expect("INSTANCE is not initialized").clone()
    }
}

static INSTANCE: OnceCell<Arc<GroupMemberService>> = OnceCell::new();

pub fn group_member_key(group_id: &str, user_id: &str) -> String {
    format!("group:member:{}:{}", group_id, user_id)
}

/// Redis: group:members:{group_id} 存储成员UID集合
pub fn group_member_list_key(group_id: &str) -> String {
    format!("group:members:{}", group_id)
}
