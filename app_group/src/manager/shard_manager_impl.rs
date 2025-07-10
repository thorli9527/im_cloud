
use tonic::async_trait;
use crate::protocol::arbitration::arbiter_service_client::ArbiterServiceClient;
use crate::protocol::arbitration::{BaseRequest, ShardState, UpdateShardStateRequest};

#[async_trait]
impl ShardManagerOpt for ShardManager {
    async fn init(&mut self) -> Result<()>  {
       self.client_init().await?;
        Ok(())
    }

    
    async fn register_node(&mut self, node_addr: &str) -> Result<()> {
        let mut client =  self.client_init().await?; 
        client.register_node(BaseRequest {
            node_addr: node_addr.to_string(),
        }).await?;
    
        Ok(())
    }
    
    async fn change_migrating(&mut self) -> Result<()> {
        let shard_address=self.shard_config.shard_address.clone();
        let mut client =  self.client_init().await?;
        let state_request = UpdateShardStateRequest { node_addr: shard_address.to_string(), new_state: ShardState::Migrating as i32 };
        client.update_shard_state(state_request).await?;
        Ok(())
    }
    
    async fn sync_groups(&mut self) -> Result<()> {
        Ok(())
    }
    
    async fn sync_group_members(&mut self) -> Result<()> {
        Ok(())
    }
    
    async fn change_preparing(&mut self) -> Result<()> {
        let shard_address=self.shard_config.shard_address.clone();
        let mut client =  self.client_init().await?;
        let state_request = UpdateShardStateRequest { node_addr: shard_address.to_string(), new_state: ShardState::Preparing as i32 };
        client.update_shard_state(state_request).await?;
        Ok(())
    }
    
    async fn change_failed(&mut self) -> Result<()> {
        let shard_address=self.shard_config.shard_address.clone();
        let mut client =  self.client_init().await?;
        let state_request = UpdateShardStateRequest { node_addr: shard_address.to_string(), new_state: ShardState::Failed as i32 };
        client.update_shard_state(state_request).await?;
        Ok(())
    }
    
    async fn change_ready(&mut self) -> Result<()> {
        let shard_address=self.shard_config.shard_address.clone();
        let mut client =  self.client_init().await?;
        let state_request = UpdateShardStateRequest { node_addr: shard_address.to_string(), new_state: ShardState::Ready as i32 };
        client.update_shard_state(state_request).await?;
        Ok(())
    }
    
    async fn change_normal(&mut self) -> Result<()> {
        let shard_address=self.shard_config.shard_address.clone();
        let mut client =  self.client_init().await?;
        let state_request = UpdateShardStateRequest { node_addr: shard_address.to_string(), new_state: ShardState::Normal as i32 };
        client.update_shard_state(state_request).await?;
        Ok(())
    }
    
    async fn change_preparing_offline(&mut self) -> Result<()> {
        let shard_address=self.shard_config.shard_address.clone();
        let mut client =  self.client_init().await?;
        let state_request = UpdateShardStateRequest { node_addr: shard_address.to_string(), new_state: ShardState::PreparingOffline as i32 };
        client.update_shard_state(state_request).await?;
        Ok(())
    }
    
    async fn change_offline(&mut self) -> Result<()> {
        let shard_address=self.shard_config.shard_address.clone();
        let mut client =  self.client_init().await?;
        let state_request = UpdateShardStateRequest { node_addr: shard_address.to_string(), new_state: ShardState::Offline as i32 };
        client.update_shard_state(state_request).await?;
        Ok(())
    }
    
    async fn heartbeat(&mut self) -> Result<()> {
        let shard_address=self.shard_config.shard_address.clone();
        let mut client =  self.client_init().await?;
        client.heartbeat(BaseRequest{node_addr: shard_address}).await?;
        Ok(())
    }
}