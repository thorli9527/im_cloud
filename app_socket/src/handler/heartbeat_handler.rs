use dashmap::DashMap;
use lazy_static::lazy_static;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tokio::sync::watch;
use tokio::time::{self, Duration};

use crate::manager::socket_manager::{ConnectionId, get_socket_manager};
use biz_service::protocol::common::ByteMessageType;
use common::util::date_util::now;
/// 每个连接对应的心跳控制器
struct ConnectionControl {
    client_timeout_stop_tx: watch::Sender<bool>,
}

lazy_static! {
    /// 保存所有连接的任务退出通知器
    static ref CONNECTION_EXIT_NOTIFY: DashMap<ConnectionId, ConnectionControl> = DashMap::new();
}
/// 启动服务端心跳发送任务 + 客户端心跳超时监控任务
pub fn start_heartbeat_tasks(
    conn_key: ConnectionId,
    last_heartbeat: Arc<AtomicU64>,
) {
    let (client_tx, client_rx) = watch::channel(false);


    tokio::spawn(start_client_timeout_checker(
        conn_key.clone(),
        last_heartbeat,
        client_rx,
    ));

    CONNECTION_EXIT_NOTIFY.insert(
        conn_key,
        ConnectionControl {
            client_timeout_stop_tx: client_tx,
        },
    );
}

/// 每 5 秒检查一次客户端最近心跳时间，如超时未收到则断开
pub async fn start_client_timeout_checker(
    conn_key: ConnectionId,
    last_heartbeat: Arc<AtomicU64>,
    mut shutdown_rx: watch::Receiver<bool>,
) {
    let mut interval = time::interval(Duration::from_secs(5));
    let manager = get_socket_manager();

    loop {
        tokio::select! {
            _ = interval.tick() => {

                let now_ts = now() as u64;
                let last = last_heartbeat.load(Ordering::Relaxed);

                if now_ts.saturating_sub(last) > 60_000 {
                    log::warn!("⏱️ 客户端心跳超时未收到，断开连接: {:?}", conn_key);
                    manager.remove(&conn_key);
                    break;
                }
            }

            Ok(_) = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    log::info!("💤 停止客户端心跳超时监控任务: {:?}", conn_key);
                    break;
                }
            }
        }
    }
}
/// 外部通知：主动停止某个连接的心跳任务
pub fn stop_heartbeat_tasks(conn_key: &ConnectionId) {
    if let Some(control) = CONNECTION_EXIT_NOTIFY.remove(conn_key).map(|e| e.1) {
        let _ = control.client_timeout_stop_tx.send(true);
    }
}





