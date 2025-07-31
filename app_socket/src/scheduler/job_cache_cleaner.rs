use crate::socket::socket_manager::SocketManager;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::time::{interval, Duration};

pub fn start() {
    let socket_manager = SocketManager::get();
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(60));
        loop {
            ticker.tick().await;
            let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;

            socket_manager.get_message_cache().retain(|_, (response, timestamp)| {
                now - *timestamp < 300_000 // 5分钟 = 300 * 1000 毫秒
            });
        }
    });
}
