use std::fs;
use std::path::PathBuf;

fn main() {
    // build_message_gateway();
    build_biz_service();
}

fn build_biz_service() {
    tonic_build::configure()
        .build_server(false) // 如无需生成 gRPC Server 代码
        .build_client(false) // 如无需生成 gRPC Client 代码
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize,utoipa::ToSchema)]")
        .type_attribute(".", "#[serde(rename_all = \"camelCase\")]")
        .out_dir("../biz_core/src/protocol/") // 输出 Rust 模块到该目录
        .compile_protos(
            &["proto/common.proto"],
            &["proto"], // proto 根目录
        )
        .expect("💥 Proto 编译失败，请检查路径和语法！");

    tonic_build::configure()
        .build_server(true) // 如无需生成 gRPC Server 代码
        .build_client(true) // 如无需生成 gRPC Client 代码
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize,utoipa::ToSchema)]")
        .type_attribute(".", "#[serde(rename_all = \"camelCase\")]")
        .out_dir("../biz_core/src/protocol/msg/") // 输出 Rust 模块到该目录
        .compile_protos(
            &[
                "proto/common.proto",
                "proto/msg/auth.proto",
                "proto/msg/friend.proto",
                "proto/msg/group.proto",
                "proto/msg/message.proto",
                "proto/msg/status.proto",
                "proto/msg/system.proto",
                "proto/msg/user.proto",
                "proto/msg/friend_server.proto",
                "proto/msg/group_server.proto",
            ],
            &["proto"], // proto 根目录
        )
        .expect("💥 Proto 编译失败，请检查路径和语法！");

    fs::remove_file("../biz_core/src/protocol/msg/common.rs").expect("删除失败");
    let out_dir = PathBuf::from("../biz_core/src/protocol/msg");
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
                let new_name = file_name.replace("protocol.", "");
                let new_path = out_dir.join(new_name);

                println!("cargo:warning=🔄 重命名 {} -> {}", file_name, new_path.display());

                if let Err(e) = fs::rename(&path, &new_path) {
                    println!("cargo:warning=⚠️ 重命名失败: {}", e);
                }
            }
        }
    }

    println!("cargo:warning=✅ proto service 编译完成！");

    tonic_build::configure()
        .build_server(true) // 如无需生成 gRPC Server 代码
        .build_client(true) // 如无需生成 gRPC Client 代码
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize,utoipa::ToSchema)]")
        .type_attribute(".", "#[serde(rename_all = \"camelCase\")]")
        .out_dir("../biz_core/src/protocol/arb/") // 输出 Rust 模块到该目录
        .compile_protos(
            &[
                "proto/common.proto",
                "proto/arb/arb_models.proto",
                "proto/arb/arb_client.proto",
                "proto/shard/shard_service.proto",
                "proto/arb/arb_server.proto",
            ],
            &["proto"], // proto 根目录
        )
        .expect("💥 Proto 编译失败，请检查路径和语法！");

    //删除 ../service/src/protocol/arb/common.rs
    fs::remove_file("../biz_core/src/protocol/arb/common.rs").expect("删除失败");

    let out_dir = PathBuf::from("../biz_core/src/protocol/arb");
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
                let new_name = file_name.replace("protocol.", "");
                let new_path = out_dir.join(new_name);

                println!("cargo:warning=🔄 重命名 {} -> {}", file_name, new_path.display());

                if let Err(e) = fs::rename(&path, &new_path) {
                    println!("cargo:warning=⚠️ 重命名失败: {}", e);
                }
            }
        }
    }

    println!("cargo:warning=✅ proto service rpc 编译完成！");
}
