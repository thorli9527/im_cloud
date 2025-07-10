# im-server

🚀 **im-server** 是一个用 Rust 编写的高性能、模块化即时通讯（IM）云中间件，支持多节点部署与水平扩展，旨在提供低延迟、可靠、高并发的消息分发服务。

## ✨ 项目特点

- 🦀 Rust 实现，内存安全，极致性能
- 🧩 多模块架构，API 接入、消息调度、群组管理、仲裁、后台任务、长连接服务等功能解耦
- ☁️ 支持多协议（REST/gRPC/WebSocket）、Redis 缓存、Kafka 消息队列、分布式分片与仲裁
- 🔄 易于扩展，适合大规模分布式 IM 场景

---

## 📁 模块结构

```bash
im-server/
├── app_api      # 对外 REST/gRPC 接口服务
├── app_socket   # 长连接管理（WebSocket/TCP），消息实时推送
├── app_job      # 异步任务/定时任务执行器
├── app_main     # 后台管理端，业务调度与聚合
├── app_group    # 群组服务，负责群组/成员管理、分片、分布式一致性
├── app_arb      # 仲裁服务，分片仲裁、健康检查、分布式一致性保障
├── biz_service  # 业务逻辑封装（用户、好友、群组、消息、MQ等）
├── common       # 公共工具库（配置、数据库、Redis、工具函数等）
├── macro        # Rust 宏定义，简化服务注册等
├── Cargo.toml   # Rust 项目依赖定义
└── README.md    # 项目说明文档
```

### 主要模块说明

- **app_api**：对外 API 网关，提供用户、消息、群组等 REST/gRPC 接口。
- **app_socket**：长连接服务，负责 WebSocket/TCP 连接管理与消息推送。
- **app_job**：定时任务、异步任务调度与执行。
- **app_main**：后台管理与业务聚合入口。
- **app_group**：群组服务，支持群组/成员管理、分片、分布式一致性， MongoDB/Redis/Kafka。
- **app_arb**：仲裁服务，负责分片健康检查、扩容、分布式一致性仲裁。
- **biz_service**：核心业务逻辑实现，包含用户、好友、群组、消息、MQ等服务。
- **common**：通用基础库，提供配置、数据库、Redis、工具方法等。
- **macro**：Rust 宏定义，提升开发效率。

---

## 🚀 快速开始

### 环境要求

- Rust ≥ 1.74（建议最新版）
- 已安装 protobuf 编译器（用于生成消息结构）
- MongoDB、Redis、Kafka（如需完整体验分布式与消息功能）

### 安装依赖

```bash
cargo build
```

### 启动服务

以主服务为例：

```bash
cargo run -p app_main
```

如需单独运行其他服务（如 API、Socket、群组、仲裁等），可分别指定：

```bash
cargo run -p app_api
cargo run -p app_socket
cargo run -p app_group
cargo run -p app_arb
```

---

## 📡 项目架构图（简化）

```text
+--------+       +---------+        +---------+
| Client | <---> | Socket  | <----> | Job     |
+--------+       +---------+        +---------+
     |               |                   |
     |         +-----v-----+      +------v-----+
     +-------> | API Layer | <--> | Biz Logic  |
               +-----------+      +------------+
                    |                   |
             +------v-----+      +------v-----+
             | Group/Arb  | <--> |  Redis/Kafka|
             +------------+      +-------------+
```

---

## 📝 开发计划

- [x] 模块拆分与基础架构搭建
- [x] 用户登录与认证机制
- [x] 群组与消息分发管理
- [x] Redis 缓存与消息存储支持
- [x] 消息队列支持（Kafka）
- [x] WebSocket 协议支持
- [x] 服务注册与健康检查（仲裁/分片）
- [ ] 更多协议与分布式特性

---

## 🤝 贡献指南

欢迎提交 PR 或 issue：

1. Fork 本项目
2. 创建功能分支：`git checkout -b feat/xxx`
3. 提交你的改动并推送
4. 创建 Pull Request

---

如需详细文档、接口说明或二次开发指导，请联系维护者或查阅源码注释。
