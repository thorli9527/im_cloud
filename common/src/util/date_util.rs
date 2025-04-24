use chrono::{DateTime, TimeZone};

///
/// 生成当前时间字符串
pub async fn build_time() -> String {
    let now = chrono::Local::now();
    now.format("%Y-%m-%d %H:%M:%S").to_string()
}
pub fn now() -> i64 {
    let now = chrono::Local::now();
    now.timestamp()
}

pub fn time_to_str(time:i64)->String{
    let t=DateTime::from_timestamp(time,0).expect("非法的时间戳");
     t.format("%Y-%m-%d %H:%M:%S").to_string()
}