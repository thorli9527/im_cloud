use std::fs;
use std::path::PathBuf;

fn main() {
    println!("cargo:warning=🔧 build.rs 正在运行...");

    // 编译 proto 文件
    tonic_build::configure()
        .build_server(false)  // 如无需生成 gRPC Server 代码
        .build_client(false)  // 如无需生成 gRPC Client 代码
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
                "proto/entity.proto",
            ],
            &["proto"], // proto 根目录
        )
        .expect("💥 Proto 编译失败，请检查路径和语法！");
    let out_dir = PathBuf::from("src/protocol");

    for entry in fs::read_dir(&out_dir).expect("无法读取目录") {
        let entry = entry.expect("无法读取文件项");
        let path = entry.path();

        if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
            // 跳过 mod.rs 和已重命名的 protocol_*.rs 文件
            if file_name == "mod.rs" || file_name.starts_with("protocol_") {
                continue;
            }

            // 匹配 protocol.xxx.rs 文件
            if file_name.starts_with("protocol.") && file_name.ends_with(".rs") {
                let new_name = file_name.replace("protocol.", "protocol_");
                let new_path = out_dir.join(new_name);

                println!("cargo:warning=🔄 重命名 {} -> {}", file_name, new_path.display());

                if let Err(e) = fs::rename(&path, &new_path) {
                    println!("cargo:warning=⚠️ 重命名失败: {}", e);
                }
            }
        }
    }
    println!("cargo:warning=✅ proto 编译完成！");
}
