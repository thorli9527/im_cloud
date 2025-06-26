fn main() {
    println!("cargo:warning=🔧 build.rs 正在运行...");

    // 编译 proto 文件
    tonic_build::configure()
        .build_server(false) // 如无需生成 gRPC Server 代码
        .build_client(false) // 如无需生成 gRPC Client 代码
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize,utoipa::ToSchema)]")
        .out_dir("src/protocol/") // 输出 Rust 模块到该目录
        .compile_protos(
            &[
                "proto/auth.proto",
                "proto/common.proto",
                "proto/envelope.proto",
                "proto/friend.proto",
                "proto/group.proto",
                "proto/message.proto",
                "proto/status.proto",
                "proto/system.proto",
                "proto/user.proto",
            ],
            &["proto"], // proto 根目录
        )
        .expect("💥 Proto 编译失败，请检查路径和语法！");

    println!("cargo:warning=✅ proto 编译完成！");
}
