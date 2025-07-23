use crate::manager::socket_manager::SocketManager;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::time::{Duration, interval};

pub async fn start_heartbeat_cleaner(manager: Arc<SocketManager>, timeout_secs: u64) {
    let mut ticker = interval(Duration::from_secs(10));

    loop {
        ticker.tick().await;
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

        let mut stale_ids = vec![];

        for item in manager.connections.iter() {
            let last = item.value().last_heartbeat.load(Ordering::Relaxed);
            if now.saturating_sub(last) > timeout_secs {
                stale_ids.push(item.key().clone());
            }
        }

        for id in stale_ids {
            manager.remove(&id);
        }
    }
}
