use crate::service::arb_manager::{ArbManagerJob, ManagerJobOpt};
use crate::service::shard_manager::{ShardManager, MEMBER_SHARD_SIZE};
use actix_web::web::get;
use biz_service::manager::common::shard_index;
use biz_service::protocol::arb::rpc_arb_models::{
    BaseRequest, MemberRef, NodeType, QueryNodeReq, ShardState, SyncListGroup, UpdateShardStateRequest,
};
use biz_service::protocol::common::GroupMemberEntity;
use chrono::format::Item;
use common::config::AppConfig;
use common::util::common_utils::hash_index;
use common::util::date_util::now;
use common::GroupId;
use dashmap::{DashMap, DashSet};
use rdkafka::groups::GroupInfo;
use std::collections::HashMap;
use std::sync::Arc;
use std::thread::current;
use tokio::sync::RwLock;
use tonic::async_trait;

#[async_trait]
impl ManagerJobOpt for ArbManagerJob {
    async fn init(&mut self) -> anyhow::Result<()> {
        self.init_arb_client().await?;
        Ok(())
    }

    async fn register_node(&mut self) -> anyhow::Result<()> {
        let config1 = &AppConfig::get().clone().shard.clone().unwrap();
        let shard_manager = ShardManager::get();
        self.shard_address = config1.clone().shard_address.unwrap();
        let client = self.init_arb_client().await?;
        let request = BaseRequest {
            node_addr: config1.clone().shard_address.unwrap(),
            node_type: NodeType::GroupNode as i32,
            socket_addr: None,
        };
        let response = client.register_node(request).await?;

        // 👇 正确使用 ArcSwap 的 load()
        let current = shard_manager.current.load();
        let mut shard_info = current.shard_info.write().await;
        self.total = response.into_inner().total as usize;
        // 设置初始状态
        shard_info.state = ShardState::Preparing;
        shard_info.last_update_time = now() as u64;
        shard_info.last_heartbeat = shard_info.last_update_time;
        Ok(())
    }

    /// 设置某群组迁移状态为“准备中”
    /// 表示目标节点已准备好接收群组（例如缓存准备、校验完成等）
    async fn change_preparing(&mut self) -> anyhow::Result<()> {
        let shard_manager = ShardManager::get();
        let current = shard_manager.current.load();
        let shard_address = self.shard_address.clone();
        let mut shard_info = current.shard_info.write().await;
        let client = self.init_arb_client().await?;
        let state_request = UpdateShardStateRequest {
            node_addr: shard_address,
            new_state: ShardState::Preparing as i32,
        };
        shard_info.state = ShardState::Preparing;
        let response = client.update_shard_state(state_request).await?;
        if response.into_inner().success {
            let current = shard_manager.current.load();
            let mut shard_info = current.shard_info.write().await;
            shard_info.state = ShardState::Preparing;
            shard_info.last_update_time = now() as u64;
            shard_info.last_heartbeat = shard_info.last_update_time;
            // ✅ 存储快照
            shard_manager.clone_current_to_snapshot();
            shard_info.state = ShardState::Migrating;
            // ✅ 清空 current
            shard_manager.clear_current();
            // ✅ 迁移状态已设置为 Preparing 开始同步数据
            if let Err(e) = self.change_migrating().await {
                log::error!("❌ sync groups error: {:?}", e);
            }

            log::info!("🔄 current 分片数据已清空，准备进入迁移流程");
        }
        Ok(())
    }
    /// 将群组分片状态设置为“迁移中”
    /// 通常意味着不再接受新写入，同时准备数据转移
    /// 将分片状态更新为 “迁移中”
    /// 表示当前分片处于不接受新写入、数据即将转移状态
    async fn change_migrating(&mut self) -> anyhow::Result<()> {
        let shard_manager = ShardManager::get();
        let shard_addr = self.shard_address.clone();
        // 1️⃣ 获取当前分片快照（snapshot）和当前结构（current）
        let current = shard_manager.current.load();
        let guard = shard_manager.snapshot.load();

        // 2️⃣ 获取当前状态写锁
        let mut shard_info = current.shard_info.write().await;

        let total = self.total;
        let node_index = hash_index(&self.shard_address, total as i32);
        for key in guard.shard_map.all_keys() {
            let hash_index = hash_index(&key, total as i32);
            if hash_index != node_index {
                continue; // 只处理当前节点的在线成员
            }
            if let Some(member_list) = guard.shard_map.get_all_items(&key) {
                current.shard_map.insert_many(&key, &member_list);
            } else {
                log::warn!("⚠️ 无法读取 snapshot 中的群组成员：{}", key);
            }
        }

        // 4️⃣ 拷贝 snapshot.online_map -> current.online_map
        for key in guard.online_map.all_keys() {
            let hash_index = hash_index(&key, total as i32);
            if hash_index != node_index {
                continue; // 只处理当前节点的在线成员
            }
            if let Some(online_list) = guard.online_map.get_all_items(&key) {
                current.online_map.insert_many(&key, &online_list);
            } else {
                log::warn!("⚠️ 无法读取 snapshot 中的在线成员：{}", key);
            }
        }

        // 5️⃣ 设置当前状态为 Migrating
        shard_info.state = ShardState::Migrating;
        shard_info.last_update_time = now() as u64;
        shard_info.last_heartbeat = shard_info.last_update_time;

        // 6️⃣ 向仲裁服务上报当前节点状态
        let client = self.init_arb_client().await?;
        let state_request = UpdateShardStateRequest {
            node_addr: shard_addr,
            new_state: ShardState::Migrating as i32,
        };

        let response = client.update_shard_state(state_request).await?;
        if response.into_inner().success {
            log::info!("✅ 节点 {} 成功设置为迁移中", self.shard_address);
        } else {
            log::warn!("⚠️ 节点 {} 设置迁移状态失败", self.shard_address);
        }

        Ok(())
    }

    async fn sync_data(&mut self) -> anyhow::Result<()> {
        let shard_manager = ShardManager::get();
        let shard_addr = self.shard_address.clone();
        let client = self.init_arb_client().await?;

        let request = QueryNodeReq {
            node_type: NodeType::GroupNode as i32,
        };
        let response = client.list_all_nodes(request).await?;
        let nodes = response.into_inner().nodes;

        let endpoints: Vec<String> = nodes.iter().map(|node| node.clone().node_addr).collect();
        let group_rpc_clients = shard_manager.init_grpc_clients(endpoints).await.expect("init grpc clients error");

        let snapshot = shard_manager.snapshot.load();
        let current = shard_manager.current.load();
        let total = self.total;
        let node_index = hash_index(&self.shard_address, total as i32);

        for key in snapshot.shard_map.all_keys() {
            let hash_index = hash_index(&key, total as i32);
            if hash_index != node_index {
                continue; // 只处理当前节点的在线成员
            }
            let on_line_member = snapshot.online_map.get_all_items(&key);

            if let Some(member_list) = snapshot.shard_map.get_all_items(&key) {
                let sync_data = SyncListGroup {
                    group_id: key,
                    members: member_list,
                    on_line_member: on_line_member.unwrap(),
                };
                let mut option = group_rpc_clients.get(&hash_index).unwrap().clone();
                option.sync_data(sync_data).await?;
            } else {
                log::warn!("⚠️ 无法读取 snapshot 中的群组成员：{}", key);
            }
        }

        let mut shard_info = current.shard_info.write().await;
        // 5️⃣ 设置当前状态为 Migrating
        shard_info.state = ShardState::Ready;
        shard_info.last_update_time = now() as u64;
        shard_info.last_heartbeat = shard_info.last_update_time;

        // 6️⃣ 向仲裁服务上报当前节点状态
        let client = self.init_arb_client().await?;
        let state_request = UpdateShardStateRequest {
            node_addr: shard_addr,
            new_state: ShardState::Ready as i32,
        };

        let response = client.update_shard_state(state_request).await?;
        if response.into_inner().success {
            log::info!("✅ 节点 {} 成功设置为迁移中", self.shard_address);
        } else {
            log::warn!("⚠️ 节点 {} 设置迁移状态失败", self.shard_address);
        }

        if let Err(e) = self.change_ready().await {
            let shard_manager = ShardManager::get();
            let current = shard_manager.current.load();
            let mut shard_info = current.shard_info.write().await;
            shard_info.state = ShardState::Syncing;
            log::error!("❌ sync groups error: {:?}", e);
        }

        Ok(())
    }

    async fn change_failed(&mut self) -> anyhow::Result<()> {
        let shard_manager = ShardManager::get();
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
        let shard_manager = ShardManager::get();
        let current = shard_manager.current.load();
        let mut shard_info = current.shard_info.write().await;
        shard_info.state = ShardState::Ready;
        let client = self.init_arb_client().await?;
        let state_request = UpdateShardStateRequest {
            node_addr: shard_address,
            new_state: ShardState::Ready as i32,
        };
        let shard_manager = ShardManager::get();
        shard_manager.clean_snapshot();
        client.update_shard_state(state_request).await?;
        self.change_normal().await?;
        Ok(())
    }

    async fn change_normal(&mut self) -> anyhow::Result<()> {
        let shard_manager = ShardManager::get();
        let current = shard_manager.current.load();
        let shard_address = self.shard_address.clone();
        let client = self.init_arb_client().await?;
        let state_request = UpdateShardStateRequest {
            node_addr: shard_address,
            new_state: ShardState::Normal as i32,
        };
        client.update_shard_state(state_request).await?;

        let mut shard_info = current.shard_info.write().await;
        shard_info.state = ShardState::Normal;
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
        let shard_manager = ShardManager::get();
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
        let request = BaseRequest {
            node_addr: shard_address,
            node_type: NodeType::GroupNode as i32,
            socket_addr: None,
        };
        client.heartbeat(request).await?;
        Ok(())
    }
}
