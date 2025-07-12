use std::thread::current;
use tonic::async_trait;
use common::util::date_util::now;
use crate::protocol::rpc_arb_models::{BaseRequest, UpdateShardStateRequest};

#[async_trait]
impl ShardManagerOpt for ShardManager {
    async fn init(&mut self) -> Result<()>  {
       self.client_init().await?;
        Ok(())
    }


    async fn register_node(&mut self) -> Result<()> {
        let node_addr = self.get_node_addr().to_string();
        let client = self.client_init().await?;

        let response=client.register_node(BaseRequest {
            node_addr: node_addr.clone(),
        }).await?;
        {
            let mut current = self.current.write().await;
            current.state = ShardState::Registered;
            current.last_heartbeat = current.last_update_time;
            current.last_update_time = now() as u64;
            let info = response.into_inner();
            self.index= info.index;
            self.total= info.total;
        }

        Ok(())
    }

    async fn change_migrating(&mut self) -> Result<()> {
        let node_addr = self.get_node_addr().to_string();
        let client = self.client_init().await?; // 👈 提前完成可变借用

        let state_request = UpdateShardStateRequest {
            node_addr,
            new_state: ShardState::Migrating as i32,
        };

        let response =client.update_shard_state(state_request).await?;
        if response.into_inner().success {
            // 更新当前分片信息
            let mut current = self.current.write().await;
            current.state = ShardState::Migrating;
            current.last_update_time = now() as u64;
            current.last_heartbeat =current.last_update_time;
        }
        Ok(())
    }
    async fn change_preparing(&mut self) -> Result<()> {
        let node_addr = self.get_node_addr().to_string();
        let client =  self.client_init().await?;
        let state_request = UpdateShardStateRequest { node_addr: node_addr, new_state: ShardState::Preparing as i32 };
        let response =client.update_shard_state(state_request).await?;
        if response.into_inner().success {
            // 获取当前分片信息
            let mut current = self.current.write().await;
            // 创建准备迁移的分片信息
            current.state = ShardState::Preparing;
            current.last_update_time = now() as u64;
            current.last_heartbeat =current.last_update_time;
        }
        Ok(())
    }
    async fn sync_groups(&mut self) -> Result<()> {
        Ok(())
    }
    
    async fn sync_group_members(&mut self) -> Result<()> {
        Ok(())
    }
    

    
    async fn change_failed(&mut self) -> Result<()> {
        let node_addr = self.get_node_addr().to_string();
        let client =  self.client_init().await?;
        let state_request = UpdateShardStateRequest { node_addr: node_addr, new_state: ShardState::Failed as i32 };
        client.update_shard_state(state_request).await?;
        Ok(())
    }
    
    async fn change_ready(&mut self) -> Result<()> {
        let node_addr = self.get_node_addr().to_string();
        let client =  self.client_init().await?;
        let state_request = UpdateShardStateRequest { node_addr: node_addr, new_state: ShardState::Ready as i32 };
        client.update_shard_state(state_request).await?;
        Ok(())
    }
    
    async fn change_normal(&mut self) -> Result<()> {
        let node_addr = self.get_node_addr().to_string();
        let client =  self.client_init().await?;
        let state_request = UpdateShardStateRequest { node_addr: node_addr, new_state: ShardState::Normal as i32 };
        client.update_shard_state(state_request).await?;
        Ok(())
    }
    
    async fn change_preparing_offline(&mut self) -> Result<()> {
        let node_addr = self.get_node_addr().to_string();
        let client =  self.client_init().await?;
        let state_request = UpdateShardStateRequest { node_addr:node_addr, new_state: ShardState::PreparingOffline as i32 };
        client.update_shard_state(state_request).await?;
        Ok(())
    }
    
    async fn change_offline(&mut self) -> Result<()> {
        let node_addr = self.get_node_addr().to_string();
        let client =  self.client_init().await?;
        let state_request = UpdateShardStateRequest { node_addr:node_addr, new_state: ShardState::Offline as i32 };
        client.update_shard_state(state_request).await?;
        Ok(())
    }
    
    async fn heartbeat(&mut self) -> Result<()> {
        let node_addr = self.get_node_addr().to_string();
        let client =  self.client_init().await?;
        client.heartbeat(BaseRequest{node_addr: node_addr}).await?;
        Ok(())
    }
}