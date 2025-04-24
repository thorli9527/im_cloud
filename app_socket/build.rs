fn main() {
    println!("cargo:warning=🔧 build.rs 正在运行...");

    tonic_build::configure()
        .build_server(false) // 如果你不使用 gRPC，可设置为 false
        .build_client(false)
        .out_dir("src/pb") // 输出路径
        .compile_protos(&["proto/im_chat.proto"], &["proto"])
        .unwrap();
}
