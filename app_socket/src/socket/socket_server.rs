use crate::kafka::kafka_consumer;
use crate::kafka::kafka_consumer::start_consumer;
use crate::manager::socket_manager::get_socket_manager;
use common::config::KafkaConfig;
use std::sync::Arc;
use tokio::net::TcpListener;
use crate::socket::socket_connection::handle_connection;

/// 启动 TCP 服务 + Kafka 消费任务
pub async fn start_server(listener: TcpListener, kafka_cfg: &KafkaConfig) -> anyhow::Result<()> {
    let socket_manager = get_socket_manager(); // ✅ 获取全局 SocketManager 单例

    // ✅ 启动 Kafka 消费者（非阻塞）
    {
        let config = kafka_cfg.clone();
        let kafka_manager = Arc::clone(&socket_manager);
        tokio::spawn(async move {
            log::info!("🚀 Kafka 消费任务启动中...");
            if let Err(e) = start_consumer(&config, kafka_manager).await {
                log::error!("❌ Kafka 消费失败: {:?}", e);
            }
        });
    }

    log::info!("✅ TCP 服务器已启动，开始监听连接...");

    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                log::info!("📡 新连接建立 [{}]", addr);

                tokio::spawn(async move {
                    if let Err(e) = handle_connection(stream).await {
                        log::error!("❌ 连接处理失败 [{}]: {:?}", addr, e);
                    } else {
                        log::info!("🔌 连接处理完成 [{}]", addr);
                    }
                });
            }
            Err(e) => {
                log::error!("❌ TCP 连接接收失败: {:?}", e);
            }
        }
    }
}
