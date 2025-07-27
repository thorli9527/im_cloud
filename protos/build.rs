use std::fs;
use std::path::PathBuf;

fn main() {
    build_arb_service();
    build_biz_service();
    build_api_service();
}

fn build_api_service() {
    tonic_build::configure()
        .build_server(false)
        .build_client(true)
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize,utoipa::ToSchema)]")
        .type_attribute(".", "#[serde(rename_all = \"camelCase\")]")
        .out_dir("../app_api/src/protocol/")
        .compile_protos(
            &["proto/common/common.proto", "proto/arb/arb_models.proto", "proto/arb/arb_server.proto"],
            &["proto"], // ✅ 设置 proto 根为 "protos"，对应 import "arb/xxx.proto"
        )
        .expect("💥 Proto 编译失败，请检查路径和语法！");

    println!("cargo:warning=✅ proto app_api 编译完成！");
}
fn build_arb_service() {
    // 编译 app_arb proto 文件
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize,utoipa::ToSchema)]")
        .type_attribute(".", "#[serde(rename_all = \"camelCase\")]")
        .out_dir("../app_arb/src/protocol/")
        .compile_protos(
            &[
                "proto/common/common.proto",
                "proto/arb/arb_models.proto",
                "proto/arb/arb_server.proto",
                "proto/arb/arb_socket.proto",
                "proto/arb/arb_group.proto",
            ],
            &["proto"], // ✅ 设置 proto 根为 "protos"，对应 import "arb/xxx.proto"
        )
        .expect("💥 Proto 编译失败，请检查路径和语法！");

    println!("cargo:warning=✅ proto app_arb 编译完成！");
}
fn build_biz_service() {
    tonic_build::configure()
        .build_server(false) // 如无需生成 gRPC Server 代码
        .build_client(false) // 如无需生成 gRPC Client 代码
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize,utoipa::ToSchema)]")
        .type_attribute(".", "#[serde(rename_all = \"camelCase\")]")
        .out_dir("../biz_service/src/protocol/") // 输出 Rust 模块到该目录
        .compile_protos(
            &[
                "proto/common/common.proto",
                "proto/msg/auth.proto",
                "proto/msg/friend.proto",
                "proto/msg/group.proto",
                "proto/msg/message.proto",
                "proto/msg/status.proto",
                "proto/msg/system.proto",
                "proto/msg/user.proto",
                "proto/msg/entity.proto",
            ],
            &["proto"], // proto 根目录
        )
        .expect("💥 Proto 编译失败，请检查路径和语法！");
    let out_dir = PathBuf::from("../biz_service/src/protocol/");

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

    println!("cargo:warning=✅ proto biz_service 编译完成！");

    tonic_build::configure()
        .build_server(true) // 如无需生成 gRPC Server 代码
        .build_client(true) // 如无需生成 gRPC Client 代码
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize,utoipa::ToSchema)]")
        .type_attribute(".", "#[serde(rename_all = \"camelCase\")]")
        .out_dir("../biz_service/src/protocol/rpc") // 输出 Rust 模块到该目录
        .compile_protos(
            &[
                "proto/common/common.proto",
                "proto/arb/arb_models.proto",
                "proto/arb/arb_group.proto",
                "proto/arb/arb_server.proto",
                "proto/arb/arb_socket.proto",
                "proto/shard/shard_service.proto",
            ],
            &["proto"], // proto 根目录
        )
        .expect("💥 Proto 编译失败，请检查路径和语法！");

    //删除 ../biz_service/src/protocol/arb/common.rs
    if let Err(e) = fs::remove_file("../biz_service/src/protocol/rpc/common.rs") {}

    println!("cargo:warning=✅ proto biz_service rpc 编译完成！");
}
