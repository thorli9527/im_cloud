# im_cloud

🚀 **im_cloud** 是一个用 Rust 编写的高性能、模块化即时通讯（IM）云中间件，支持多节点部署与水平扩展，旨在提供低延迟、可靠、高并发的消息分发服务。

## ✨ 项目特点

- 🦀 使用 Rust 实现，内存安全，性能强大
- 📦 多模块架构：支持 API 接入、消息调度、后台任务、长连接服务等功能解耦
- 🧩 设计灵感来源于分布式 IM 架构，如 [im-cloud（PHP）](https://github.com/brewlin/im-cloud)
- ☁️ 未来支持多协议（TCP/WebSocket/gRPC）、Redis 中间件、任务队列、集群调度等功能

---

## 📁 模块结构

```bash
im_cloud/
├── app_api         # 对外 REST/gRPC 接口服务
├── app_socket      # 长连接管理模块（如 TCP/WebSocket 服务端）
├── app_job         # 异步任务/定时任务执行器
├── app_main        # 项目后台管理端，整合业务调度
├── biz_service     # 业务逻辑处理封装
├── common          # 公共工具库，如日志、配置、错误处理
├── macro           # Rust 宏定义，用于简化服务注册等
├── proto           # Protocol Buffers 消息定义
├── web_api/        # 前端调试页面（可选）
├── Cargo.toml      # Rust 项目依赖定义
└── rustfmt.toml    # 格式化配置


🚀 快速开始
🛠 环境要求
Rust ≥ 1.74（建议使用最新版）

已安装 protobuf 编译器（用于生成消息结构）

📦 安装依赖
bash
复制
编辑
cargo build
🧪 启动服务
以 app_main 启动为例：

bash
复制
编辑
cargo run -p app_main
如需单独运行其他服务（如 API 或 socket），可分别指定：

bash
复制
编辑
cargo run -p app_api
cargo run -p app_socket
📡 项目架构图（草案）
text
复制
编辑
+--------+       +---------+        +---------+
| Client | <---> | Socket  | <----> | Job     |
+--------+       +---------+        +---------+
     |               |                   |
     |         +-----v-----+      +------v-----+
     +-------> | API Layer | <--> | Biz Logic  |
               +-----------+      +------------+
✅ 开发计划
 模块拆分与基础架构搭建

 用户登录与认证机制

 群组与消息分发管理

 Redis 缓存与消息存储支持

 消息队列支持（如 Kafka / NATS）

 WebSocket 协议支持

 服务注册与健康检查（如 etcd/consul）

🤝 贡献指南
欢迎提交 PR 或 issue：

Fork 本项目

创建功能分支：git checkout -b feat/xxx

提交你的改动并推送

创建 Pull Request
