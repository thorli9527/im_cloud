use std::path::PathBuf;

fn main() {
    println!("cargo:warning=🔧 build.rs 正在运行...");

    use std::path::PathBuf;

    // 编译 proto 文件
    tonic_build::configure()
        .build_server(false) // 如无需生成 gRPC Server 代码
        .build_client(false) // 如无需生成 gRPC Client 代码
        .type_attribute(
            ".",
            "#[derive(serde::Serialize, serde::Deserialize,utoipa::ToSchema)]",
        )
        .out_dir("service/src/protocol/") // 输出 Rust 模块到该目录
        .compile_protos(
            &[
                "proto/auth.proto",
                "proto/common.proto",
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
    let out_dir = PathBuf::from("src/protocol/");

   
    println!("cargo:warning=✅ proto 编译 service 完成！");

    //
    // // 编译 app_arb proto 文件
    // tonic_build::configure()
    //     .build_server(true)
    //     .build_client(false)
    //     .type_attribute(
    //         ".",
    //         "#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]",
    //     )
    //     .out_dir("app_arb/src/protocol/")
    //     .compile_protos(
    //         &[
    //             "arb/models_arb.proto",
    //             "arb/arb_server.proto"
    //         ],
    //         &["protos"] // ✅ 设置 proto 根为 "protos"，对应 import "arb/xxx.proto"
    //     )
    //     .expect("💥 Proto 编译失败，请检查路径和语法！");
    //
    // tonic_build::configure()
    //     .build_server(false) // 如无需生成 gRPC Server 代码
    //     .build_client(true) // 如无需生成 gRPC Client 代码
    //     .type_attribute(
    //         ".",
    //         "#[derive(serde::Serialize, serde::Deserialize,utoipa::ToSchema)]",
    //     )
    //     .out_dir("app_group/src/protocol/") // 输出 Rust 模块到该目录
    //     .compile_protos(
    //         &[
    //             "arb/models_arb.proto",
    //             "arb/arb_server.proto"],
    //         &["arb"], // proto 根目录
    //     )
    //     .expect("💥 Proto 编译失败，请检查路径和语法！");
    //
    // println!("cargo:warning=✅ proto app_arb 编译完成！");
    //
    // tonic_build::configure()
    //     .build_server(true) // 如无需生成 gRPC Server 代码
    //     .build_client(false) // 如无需生成 gRPC Client 代码
    //     .type_attribute(
    //         ".",
    //         "#[derive(serde::Serialize, serde::Deserialize,utoipa::ToSchema)]",
    //     )
    //     .out_dir("../app_group/src/protocol/") // 输出 Rust 模块到该目录
    //     .compile_protos(
    //         &[
    //             "arb/models_arb.proto",
    //             "arb/arb_server.proto"],
    //         &["arb"], // proto 根目录
    //     )
    //     .expect("💥 Proto 编译失败，请检查路径和语法！");
    //
    //
    // tonic_build::configure()
    //     .build_server(false) // 如无需生成 gRPC Server 代码
    //     .build_client(true) // 如无需生成 gRPC Client 代码
    //     .type_attribute(
    //         ".",
    //         "#[derive(serde::Serialize, serde::Deserialize,utoipa::ToSchema)]",
    //     )
    //     .out_dir("../app_arb/src/protocol/") // 输出 Rust 模块到该目录
    //     .compile_protos(
    //         &[
    //             "arb/models_arb.proto",
    //             "arb/arb_server.proto"],
    //         &["arb"], // proto 根目录
    //     )
    //     .expect("💥 Proto 编译失败，请检查路径和语法！");

    println!("cargo:warning=✅ proto app_group 编译完成！");
}
