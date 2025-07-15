use crate::manager::shard_job::{ManagerJob, ManagerJobOpt};
use crate::manager::shard_manager::{MEMBER_SHARD_SIZE, MemData, ShardInfo, ShardManager};
use crate::protocol::rpc_arb_models::{
    BaseRequest, MemberRef, ShardState, SyncListGroup, UpdateShardStateRequest,
};
use actix_web::web::get;
use biz_service::manager::common::shard_index;
use common::GroupId;
use common::util::common_utils::hash_index;
use common::util::date_util::now;
use dashmap::{DashMap, DashSet};
use rdkafka::groups::GroupInfo;
use std::collections::HashMap;
use std::sync::Arc;
use std::thread::current;
use tokio::sync::RwLock;
use tonic::async_trait;
use biz_service::protocol::common::GroupMemberEntity;

pub struct RPCSyncData {
    // 群组信息
    group_info: GroupInfo,
    // 群组数据
    member: Vec<GroupMemberEntity>,
}
#[async_trait]
impl ManagerJobOpt for ManagerJob {
    async fn init(&mut self) -> anyhow::Result<()> {
        self.init_arb_client().await?;
        Ok(())
    }

    async fn register_node(&mut self) -> anyhow::Result<()> {
        let shard_manager = ShardManager::get();
        let shard_address = self.shard_address.clone();
        let client = self.init_arb_client().await?;

        let response = client
            .register_node(BaseRequest {
                node_addr: shard_address,
            })
            .await?;

        // 👇 正确使用 ArcSwap 的 load()
        let current = shard_manager.current.load();
        let mut shard_info = current.shard_info.write().await;

        // 设置初始状态
        shard_info.state = ShardState::Registered;
        shard_info.last_update_time = now() as u64;
        shard_info.last_heartbeat = shard_info.last_update_time;

        let info = response.into_inner();
        shard_info.index = hash_index(&info.node_addr, info.total);
        shard_info.total = info.total;

        Ok(())
    }

    /// 设置某群组迁移状态为“准备中”
    /// 表示目标节点已准备好接收群组（例如缓存准备、校验完成等）
    async fn change_preparing(&mut self) -> anyhow::Result<()> {
        let shard_manager = ShardManager::get();
        let shard_address = self.shard_address.clone();
        let client = self.init_arb_client().await?;
        let state_request = UpdateShardStateRequest {
            node_addr: shard_address,
            new_state: ShardState::Preparing as i32,
        };

        let response = client.update_shard_state(state_request).await?;
        if response.into_inner().success {
            let current = shard_manager.current.load();
            let mut shard_info = current.shard_info.write().await;
            shard_info.state = ShardState::Preparing;
            shard_info.last_update_time = now() as u64;
            shard_info.last_heartbeat = shard_info.last_update_time;
            // ✅ 存储快照
            shard_manager.clone_current_to_snapshot();
            // ✅ 清空 current
            shard_manager.clear_current();
            log::info!("🔄 current 分片数据已清空，准备进入迁移流程");
        }
        Ok(())
    }
    /// 将群组分片状态设置为“迁移中”
    /// 通常意味着不再接受新写入，同时准备数据转移
    async fn change_migrating(&mut self) -> anyhow::Result<()> {
        let shard_manager = ShardManager::get();
        let shard_address = self.shard_address.clone();
        let client = self.init_arb_client().await?;

        // 1. 设置分片状态为 Migrating
        let state_request = UpdateShardStateRequest {
            node_addr: shard_address,
            new_state: ShardState::Migrating as i32,
        };
        let response = client.update_shard_state(state_request).await?;

        if response.into_inner().success {
            let current = shard_manager.current.load();
            let mut shard_info = current.shard_info.write().await;
            shard_info.state = ShardState::Migrating;
            shard_info.last_update_time = now() as u64;
            shard_info.last_heartbeat = shard_info.last_update_time;
        }

        // 2. 快照数据迁移到 current
        let snapshot = shard_manager.snapshot.load();
        let current = shard_manager.current.load();
        let current_index = current.shard_info.read().await.index;

        let mut migrated_group_count = 0;

        for shard_entry in snapshot.group_shard_map.iter() {
            let (_, group_map) = shard_entry.pair();

            for group_entry in group_map.iter() {
                let group_id = group_entry.key();
                let group_index = shard_manager.hash_group_id(group_id) as i32;

                // 仅迁移属于当前分片的群组
                if group_index != current_index {
                    continue;
                }

                let shard_key = format!("shard_{}", group_index);

                // === group_shard_map ===
                current
                    .group_shard_map
                    .entry(shard_key.clone())
                    .or_insert_with(DashMap::new)
                    .insert(group_id.clone(), ());

                // === group_member_map（Vec<DashSet<UserId>>）===
                if let Some(member_map) = snapshot.group_member_map.get(&shard_key) {
                    if let Some(member_shards) = member_map.get(group_id) {
                        let target_group_map = current
                            .group_member_map
                            .entry(shard_key.clone())
                            .or_insert_with(DashMap::new);

                        let target_shards =
                            target_group_map.entry(group_id.clone()).or_insert_with(|| {
                                (0..MEMBER_SHARD_SIZE)
                                    .map(|_| DashSet::new())
                                    .collect::<Vec<_>>()
                            });

                        for (i, shard) in member_shards.iter().enumerate() {
                            if i == current_index as usize {
                                for user in shard.iter().map(|u| u.key().clone()) {
                                    target_shards[i].insert(user);
                                }
                            }
                        }
                    }
                }

                // === group_online_member_map ===
                if let Some(online_map) = snapshot.group_online_member_map.get(&shard_key) {
                    if let Some(online_set) = online_map.get(group_id) {
                        let target_group_map = current
                            .group_online_member_map
                            .entry(shard_key.clone())
                            .or_insert_with(DashMap::new);
                        let target_user_set = target_group_map
                            .entry(group_id.clone())
                            .or_insert_with(DashSet::new);

                        for user in online_set.iter().map(|u| u.key().clone()) {
                            let index = shard_manager.hash_group_member_id(group_id, &user);
                            if index == current_index as usize {
                                target_user_set.insert(user);
                            }
                        }
                    }
                }

                migrated_group_count += 1;
            }
        }

        log::info!(
            "✅ 共迁移群组 {} 个至当前分片 shard_{}",
            migrated_group_count,
            current_index
        );

        Ok(())
    }

    async fn sync_data(&mut self) -> anyhow::Result<()> {
        // === Step 1: 获取仲裁服务客户端并列出所有节点 ===
        let client = self.init_arb_client().await?;
        let response = client.list_all_nodes(()).await?;
        let nodes = response.into_inner().nodes;

        let endpoints: Vec<String> = nodes.iter().map(|node| node.clone().node_addr).collect();
        // === Step 2: 初始化 gRPC 客户端连接 ===
        let mut group_rpc_clients = self
            .init_grpc_clients(endpoints)
            .await
            .expect("init grpc clients error");

        // === Step 3: 获取当前快照状态 ===
        let shard_manager = ShardManager::get();
        let current = shard_manager.snapshot.load();

        let guard = current.shard_info.read().await;
        let current_index = guard.index;
        let total_shards = guard.total;

        // === Step 4: 构造群组分布表 ===
        let mut shard_group_map: HashMap<i32, Vec<GroupId>> = HashMap::new();
        for shard_entry in current.group_shard_map.iter() {
            let (_, group_map) = shard_entry.pair();
            for group_entry in group_map.iter() {
                let group_id = group_entry.key();
                let shard_index = hash_index(group_id, total_shards);
                if shard_index == current_index {
                    continue;
                }
                shard_group_map
                    .entry(shard_index)
                    .or_insert_with(Vec::new)
                    .push(group_id.clone());
            }
        }

        // === Step 5: 构造成员分布表 ===
        let mut shard_member_map: HashMap<i32, Vec<MemberRef>> = HashMap::new();
        for shard_entry in current.group_member_map.iter() {
            let (_, group_map) = shard_entry.pair();
            for group_entry in group_map.iter() {
                let (group_id, member_shards) = group_entry.pair();
                let expected_index = hash_index(group_id, total_shards);
                if expected_index == current_index {
                    continue;
                }

                for shard in member_shards.iter() {
                    for member in shard.iter() {
                        let uid = member.key();
                        shard_member_map
                            .entry(expected_index)
                            .or_insert_with(Vec::new)
                            .push(MemberRef {
                                group_id: group_id.clone(),
                                uid: uid.clone(),
                            });
                    }
                }
            }
        }

        // === Step 6: 并发同步数据到其它节点（含分片与限批） ===
        for (shard_index, (index, client)) in group_rpc_clients.iter_mut().enumerate() {
            let shard_index = shard_index as i32;
            if shard_index == current_index {
                continue;
            }

            let mut tasks = vec![];

            // 同步 group 分片数据（分批次，每批最多 5000）
            if let Some(groups) = shard_group_map.get(&shard_index) {
                for chunk in groups.chunks(5000) {
                    let mut client = client.clone();
                    let req = SyncListGroup {
                        groups: chunk.to_vec(),
                        members: vec![]
                    };
                    tasks.push(tokio::spawn(async move {
                        match client.sync_data(req).await {
                            Ok(_) => log::info!("✅ 同步群组至 shard_{} 成功", shard_index),
                            Err(e) => log::error!("❌ 同步群组至 shard_{} 失败: {:?}", shard_index, e),
                        }
                    }));
                }
            }

            // 同步 member 分片数据（分批次，每批最多 5000）
            if let Some(members) = shard_member_map.get(&shard_index) {
                for chunk in members.chunks(5000) {
                    let mut client = client.clone();
                    let req = SyncListGroup {
                        groups: vec![],
                        members: chunk.to_vec(),
                    };
                    tasks.push(tokio::spawn(async move {
                        match client.sync_data(req).await {
                            Ok(_) => log::info!("✅ 同步成员至 shard_{} 成功", shard_index),
                            Err(e) => log::error!("❌ 同步成员至 shard_{} 失败: {:?}", shard_index, e),
                        }
                    }));
                }
            }

            // 等待该目标分片的所有任务完成
            for task in tasks {
                let _ = task.await;
            }
        }
        Ok(())
    }

    async fn change_failed(&mut self) -> anyhow::Result<()> {
        let shard_address = self.shard_address.clone();
        let client = self.init_arb_client().await?;
        let state_request = UpdateShardStateRequest {
            node_addr: shard_address,
            new_state: ShardState::Failed as i32,
        };
        client.update_shard_state(state_request).await?;
        Ok(())
    }

    async fn change_ready(&mut self) -> anyhow::Result<()> {
        let shard_address = self.shard_address.clone();
        let client = self.init_arb_client().await?;
        let state_request = UpdateShardStateRequest {
            node_addr: shard_address,
            new_state: ShardState::Ready as i32,
        };
        let shard_manager = ShardManager::get();
        shard_manager.clean_snapshot();
        client.update_shard_state(state_request).await?;
        Ok(())
    }

    async fn change_normal(&mut self) -> anyhow::Result<()> {
        let shard_address = self.shard_address.clone();
        let client = self.init_arb_client().await?;
        let state_request = UpdateShardStateRequest {
            node_addr: shard_address,
            new_state: ShardState::Normal as i32,
        };
        client.update_shard_state(state_request).await?;
        Ok(())
    }

    async fn change_preparing_offline(&mut self) -> anyhow::Result<()> {
        let shard_address = self.shard_address.clone();
        let client = self.init_arb_client().await?;
        let state_request = UpdateShardStateRequest {
            node_addr: shard_address,
            new_state: ShardState::PreparingOffline as i32,
        };
        client.update_shard_state(state_request).await?;
        Ok(())
    }

    async fn change_offline(&mut self) -> anyhow::Result<()> {
        let shard_address = self.shard_address.clone();
        let client = self.init_arb_client().await?;
        let state_request = UpdateShardStateRequest {
            node_addr: shard_address,
            new_state: ShardState::Offline as i32,
        };
        client.update_shard_state(state_request).await?;
        Ok(())
    }

    async fn heartbeat(&mut self) -> anyhow::Result<()> {
        let shard_address = self.server_host.clone();
        let client = self.init_arb_client().await?;
        client
            .heartbeat(BaseRequest {
                node_addr: shard_address,
            })
            .await?;
        Ok(())
    }
}
