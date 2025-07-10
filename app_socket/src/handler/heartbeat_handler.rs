use std::sync::atomic::Ordering;
use tokio::time::{self, Duration, Interval};
use common::util::date_util::now;
use crate::manager::socket_manager::get_socket_manager;

/// 启动全局统一心跳检测任务
pub fn start_global_heartbeat_checker() {
    tokio::spawn(async {
        let manager = get_socket_manager();
        let mut interval = time::interval(Duration::from_secs(1));
        loop {
            interval.tick().await;

            let now_ts = now() as u64;

            // 遍历当前所有连接
            for entry in manager.connections.iter() {
                let conn_id = entry.key();
                let last_heartbeat = entry.value().last_heartbeat.load(Ordering::Relaxed);
                if now_ts.saturating_sub(last_heartbeat) > 60_000 {
                    log::warn!("⏱️ 心跳超时: {:?}", conn_id);
                    manager.remove(conn_id);
                }
            }
        }
    });
}