use crate::manager::shard_job::{ManagerJob, ManagerJobOpt};
use crate::manager::shard_manager::{MemData, ShardInfo, ShardManager};
use crate::protocol::rpc_arb_models::{BaseRequest, ShardState, UpdateShardStateRequest};
use common::util::common_utils::hash_index;
use common::util::date_util::now;
use std::sync::Arc;
use std::thread::current;
use tokio::sync::RwLock;
use tonic::async_trait;

#[async_trait]
impl ManagerJobOpt for ManagerJob {
    async fn init(&mut self) -> anyhow::Result<()> {
        self.client_init().await?;
        Ok(())
    }

    async fn register_node(&mut self) -> anyhow::Result<()> {
        let shard_manager = ShardManager::get();
        let shard_address = self.shard_address.clone();
        let client = self.client_init().await?;

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
        let client = self.client_init().await?;
        let state_request = UpdateShardStateRequest {
            node_addr: shard_address,
            new_state: ShardState::Preparing as i32,
        };

        let response = client.update_shard_state(state_request).await?;
        if response.into_inner().success {
            let current = shard_manager.current.load();

            {
                let mut shard_info = current.shard_info.write().await;
                shard_info.state = ShardState::Preparing;
                shard_info.last_update_time = now() as u64;
                shard_info.last_heartbeat = shard_info.last_update_time;
            }

            // ✅ 存储快照
            shard_manager.snapshot.store(current.clone());

            // ✅ 清空 current
            shard_manager.current.store(Arc::new(MemData {
                group_shard_map: Default::default(),
                group_member_map: Default::default(),
                group_online_member_map: Default::default(),
                shard_info: RwLock::new(ShardInfo::default()),
            }));

            log::info!("🔄 current 分片数据已清空，准备进入迁移流程");
        }
        Ok(())
    }
    /// 将群组分片状态设置为“迁移中”
    /// 通常意味着不再接受新写入，同时准备数据转移
    async fn change_migrating(&mut self) -> anyhow::Result<()> {
        let shard_manager = ShardManager::get();
        let shard_address=self.shard_address.clone();
        let client = self.client_init().await?; // 👈 提前完成可变借用

        let state_request = UpdateShardStateRequest {
            node_addr:shard_address,
            new_state: ShardState::Migrating as i32,
        };

        let response = client.update_shard_state(state_request).await?;
        if response.into_inner().success {
            // 更新当前分片信息
            let guard = shard_manager.current.load();
            let mut shard_info = guard.shard_info.write().await;
            shard_info.state = ShardState::Migrating;
            shard_info.last_update_time = now() as u64;
            shard_info.last_heartbeat = shard_info.last_update_time;
        }
        Ok(())
    }

    async fn sync_groups(&mut self) -> anyhow::Result<()> {
        
        Ok(())
    }

    async fn sync_group_members(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    async fn change_failed(&mut self) -> anyhow::Result<()> {
        let shard_address=self.shard_address.clone();
        let client = self.client_init().await?;
        let state_request = UpdateShardStateRequest {
            node_addr: shard_address,
            new_state: ShardState::Failed as i32,
        };
        client.update_shard_state(state_request).await?;
        Ok(())
    }

    async fn change_ready(&mut self) -> anyhow::Result<()> {
        let shard_address=self.shard_address.clone();
        let client = self.client_init().await?;
        let state_request = UpdateShardStateRequest {
            node_addr: shard_address,
            new_state: ShardState::Ready as i32,
        };
        client.update_shard_state(state_request).await?;
        Ok(())
    }

    async fn change_normal(&mut self) -> anyhow::Result<()> {
        let shard_address=self.shard_address.clone();
        let client = self.client_init().await?;
        let state_request = UpdateShardStateRequest {
            node_addr: shard_address,
            new_state: ShardState::Normal as i32,
        };
        client.update_shard_state(state_request).await?;
        Ok(())
    }

    async fn change_preparing_offline(&mut self) -> anyhow::Result<()> {
        let shard_address=self.shard_address.clone();
        let client = self.client_init().await?;
        let state_request = UpdateShardStateRequest {
            node_addr: shard_address,
            new_state: ShardState::PreparingOffline as i32,
        };
        client.update_shard_state(state_request).await?;
        Ok(())
    }

    async fn change_offline(&mut self) -> anyhow::Result<()> {
        let shard_address=self.shard_address.clone();
        let client = self.client_init().await?;
        let state_request = UpdateShardStateRequest {
            node_addr: shard_address,
            new_state: ShardState::Offline as i32,
        };
        client.update_shard_state(state_request).await?;
        Ok(())
    }

    async fn heartbeat(&mut self) -> anyhow::Result<()> {
        let shard_address=self.server_host.clone();
        let client = self.client_init().await?;
        client
            .heartbeat(BaseRequest {
                node_addr: shard_address,
            })
            .await?;
        Ok(())
    }
}
