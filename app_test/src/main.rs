#[tokio::main]
async fn main() {
    let handle = tokio::spawn(async {
        println!("正在异步执行");
        42
    });

    // 等待执行完成
    let result = handle.await.unwrap();
    println!("返回结果：{}", result);
}
