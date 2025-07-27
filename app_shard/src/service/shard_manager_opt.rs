use crate::service::arb_manager::ArbManagerJob;
use crate::service::shard_manager::{ShardManager, ShardManagerOpt};
use anyhow::Result;
use async_trait::async_trait;
use biz_service::biz_service::group_member_service::GroupMemberService;
use biz_service::biz_service::group_service::GroupService;
use biz_service::protocol::common::{GroupMemberEntity, GroupRoleType};
use common::config::AppConfig;
use common::util::common_utils::hash_index;
use common::{GroupId, UserId};
use futures_util::StreamExt;
use mongodb::bson::doc;
use mongodb::options::FindOptions;
use biz_service::protocol::rpc::rpc_arb_models::{MemberRef, NodeType, QueryNodeReq};

#[async_trait]
impl ShardManagerOpt for ShardManager {
    async fn load_from_data(&self) -> anyhow::Result<()> {
        let group_service = GroupService::get();
        let group_member_service = GroupMemberService::get();
        let collection = &group_service.dao.collection;
        let page_size = 100;
        let mut page = 0;
        let mut arb_manager_job = ArbManagerJob::new();
        arb_manager_job.init_arb_client().await?;
        let req = QueryNodeReq {
            node_type: NodeType::GroupNode as i32,
        };
        let response = arb_manager_job.arb_client.unwrap().list_all_nodes(req).await?;

        let list = response.get_ref();

        let shard_addr = &AppConfig::get().shard.clone().unwrap().shard_address.unwrap();

        let shard_index = hash_index(shard_addr, list.nodes.len() as i32);

        loop {
            let skip = page * page_size;

            let find_options = FindOptions::builder()
                .projection(doc! { "_id": 1 }) // 只取 _id
                .limit(page_size as i64)
                .skip(skip as u64)
                .build();

            let mut cursor = collection.find(doc! {}).with_options(find_options).await?;

            let mut has_result = false;

            while let Some(doc) = cursor.next().await {
                has_result = true;

                // 获取 group_id
                let group_id = match doc {
                    Ok(d) => match d.get_object_id("_id") {
                        Ok(oid) => oid.to_hex(),
                        Err(_) => continue,
                    },
                    Err(_) => continue,
                };
                let group_index = hash_index(&group_id, list.nodes.len() as i32);
                if shard_index != group_index {
                    continue;
                }
                // 查询该群的所有成员
                let members: Vec<GroupMemberEntity> = group_member_service.get_all_members_by_group_id(&group_id).await?;

                // 将每个成员添加到该群组分片中
                for member in members {
                    let role_type = GroupRoleType::try_from(member.role)?;
                    self.add_member(&group_id, &member.uid, role_type)?;
                }
            }
            if !has_result {
                break;
            }
            page += 1;
        }

        Ok(())
    }

    async fn create(&self, group_id: &str) -> anyhow::Result<()> {
        let group_member = GroupMemberService::get();
        let members = group_member.get_all_members_by_group_id(group_id).await?;
        for member in members {
            let result = GroupRoleType::try_from(member.role)?;
            self.add_member(group_id, &member.uid, result)?;
        }
        Ok(())
    }

    fn dismiss(&self, group_id: &str) {
        self.current.load().shard_map.clear(group_id)
    }

    /// 计算群组分片索引（用于分配 group → shard）
    fn add_member(&self, group_id: &str, uid: &UserId, role: GroupRoleType) -> Result<()> {
        let member_ref = MemberRef {
            id: uid.clone(),
            role: role as i32,
        };
        self.current.load().shard_map.insert(group_id.to_string(), member_ref);
        Ok(())
    }

    /// 从指定群组中移除某个用户（自动计算分片）
    fn remove_member(&self, group_id: &GroupId, uid: &UserId) -> anyhow::Result<()> {
        self.current.load().shard_map.remove(group_id, uid);
        Ok(())
    }

    /// 获取某个群组的所有成员 ID 列表
    fn get_member(&self, group_id: &GroupId) -> Result<Vec<MemberRef>> {
        let member_list = self.current.load().shard_map.get_member_by_key(group_id);
        return Ok(member_list);
    }

    /// 获取群组成员分页列表
    fn get_member_page(&self, group_id: &GroupId, offset: usize, limit: usize) -> Result<Option<Vec<MemberRef>>> {
        let result = self.current.load().shard_map.get_page(group_id, offset, limit);
        return Ok(result);
    }
    fn get_member_count(&self, group_id: &GroupId) -> Result<usize> {
        let count = self.current.load().shard_map.get_member_count_by_key(group_id);
        return Ok(count as usize);
    }
    fn online(&self, group_id: &GroupId, uid: &UserId) -> Result<()> {
        self.current.load().shard_map.set_online(group_id, uid, true);
        return Ok(());
    }

    fn offline(&self, group_id: &GroupId, user_id: &UserId) {
        self.current.load().shard_map.set_online(group_id, user_id, false);
    }
    fn get_on_line_member(&self, group_id: &GroupId) -> Vec<UserId> {
        self.current.load().shard_map.get_online_ids(group_id)
    }

    fn change_role(&self, group_id: &GroupId, uid: &UserId, role: GroupRoleType) -> Result<()> {
        self.current.load().shard_map.change_role(group_id, uid, role);
        Ok(())
    }
    async fn get_admin_member(&self, group_id: &GroupId) -> anyhow::Result<Option<Vec<UserId>>> {
        let group_service = GroupService::get();
        let admin_member = group_service.query_admin_member_by_group_id(group_id).await?;
        Ok(Some(admin_member))
    }
}
