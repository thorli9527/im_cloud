use chrono::{DateTime, Local, Offset};

///
/// 生成当前时间字符串
pub async fn build_time() -> String {
    let now = chrono::Local::now();
    now.format("%Y-%m-%d %H:%M:%S").to_string()
}
///
/// 返回当前时间毫秒
pub fn now() -> i64 {
    let now = chrono::Local::now();
    now.timestamp_millis()
}

pub fn time_to_str(mut time: i64) -> String {
    time = time / 1000;
    let t = DateTime::from_timestamp(time, 0).expect("非法的时间戳");
    t.format("%Y-%m-%d %H:%M:%S").to_string()
}
pub fn date_str_to_time(date_str: &str) -> i64 {
    let offset = Local::now().offset().fix(); // 当前系统偏移，例如 +08:00
    let date_with_offset = format!("{} {}", date_str, offset); // 拼接偏移

    let dt = DateTime::parse_from_str(&date_with_offset, "%Y-%m-%d %H:%M:%S %z")
        .expect("日期字符串格式错误");
    dt.timestamp_millis()
}
