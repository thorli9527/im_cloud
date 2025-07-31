use chrono::{DateTime, Utc};
use common::util::common_utils::SafeSnowflake;
use md5::Digest;
use std::hash::Hash;
use std::time::{Duration, UNIX_EPOCH};
#[tokio::main]
async fn main() {
    let mut generator = SafeSnowflake::new(1, 1);
    let message_id = generator.generate();
    let timestamp = SafeSnowflake::extract_timestamp(message_id);

    // 转换为可读时间
    let time = UNIX_EPOCH + Duration::from_millis(timestamp);
    let datetime = DateTime::<Utc>::from(time);
    println!("生成的雪花ID: {}", message_id);
    println!("提取的时间戳: {}", timestamp);
    println!("可读时间: {}", datetime.format("%Y-%m-%d %H:%M:%S%.3f"));
}
