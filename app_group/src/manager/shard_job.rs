use tokio_util::sync::CancellationToken;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::manager::shard_manager::{ShardManager, ShardManagerOpt};

pub struct ManagerJob {
    pub manager: Arc<Mutex<ShardManager>>,
    cancel_token: CancellationToken,
}

impl ManagerJob {
    pub fn new(manager: Arc<Mutex<ShardManager>>) -> Self {
        Self {
            manager,
            cancel_token: CancellationToken::new(),
        }
    }

    /// 启动心跳和生命周期任务
    pub async fn start(&self) {
        // 初始化
        {
            let mut job = self.manager.lock().await;
            if let Err(e) = job.init().await {
                eprintln!("[init] error: {:?}", e);
                return;
            }
        }

        let lifecycle_token = self.cancel_token.clone();
        let manager_clone = self.manager.clone();

        // 生命周期自动流转任务
        tokio::spawn(async move {
            if lifecycle_token.is_cancelled() {
                return;
            }
            let mut mgr = manager_clone.lock().await;
            if let Err(e) = mgr.register_node().await {
                eprintln!("[lifecycle] register_node error: {:?}", e);
                return;
            }
            if let Err(e) = mgr.change_preparing().await {
                eprintln!("[lifecycle] change_preparing error: {:?}", e);
                return;
            }
            if let Err(e) = mgr.sync_groups().await {
                eprintln!("[lifecycle] sync_groups error: {:?}", e);
                return;
            }
            if let Err(e) = mgr.sync_group_members().await {
                eprintln!("[lifecycle] sync_group_members error: {:?}", e);
                return;
            }
            if let Err(e) = mgr.change_ready().await {
                eprintln!("[lifecycle] change_ready error: {:?}", e);
                return;
            }
            if let Err(e) = mgr.change_normal().await {
                eprintln!("[lifecycle] change_normal error: {:?}", e);
            }
        });

        // 启动心跳任务
        let heartbeat_token = self.cancel_token.clone();
        let heartbeat_manager = self.manager.clone();
        tokio::spawn(async move {
            loop {
                if heartbeat_token.is_cancelled() {
                    println!("[heartbeat] stopped");
                    break;
                }

                {
                    let mut mgr = heartbeat_manager.lock().await;
                    if let Err(e) = mgr.heartbeat().await {
                        eprintln!("heartbeat error: {:?}", e);
                    }
                }

                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
        });
    }

    /// 停止所有任务
    pub fn stop(&self) {
        self.cancel_token.cancel();
    }
}
