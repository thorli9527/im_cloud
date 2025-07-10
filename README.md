项目整体结构
你的项目名为 im-server，从目录和文件命名来看，这是一个即时通讯（IM）服务端项目，采用 Rust 语言开发，采用了多模块/多服务架构。项目包含多个子应用（app_开头）、业务服务（biz_service）、通用库（common）、宏定义（macro）等。
1. 各子项目/模块说明
1.1 app_api
作用：API 网关或 HTTP 接口服务，负责对外提供 RESTful API。
主要目录/文件：
src/handlers/：各类业务处理器（如好友、群组、消息、用户等）。
static/swagger-ui/：API 文档页面。
result.rs：统一的接口返回结构。
1.2 app_arb
作用：仲裁服务，可能用于消息或事务的仲裁处理。
主要目录/文件：
proto/arb_service.proto：gRPC 协议定义。
src/biz_service/grpc/：gRPC 服务实现。
src/protocol/：协议相关实现。
1.3 app_group
作用：群组服务，负责群组相关的业务逻辑。
主要目录/文件：
proto/：群组相关的 proto 定义。
src/biz_service/：群组和群成员服务。
src/manager/：分片管理器，可能用于群组分布式管理。
src/protocol/：协议实现。
1.4 app_job
作用：定时任务或异步任务服务。
主要目录/文件：
src/main.rs：主入口。
1.5 app_main
作用：主服务，可能是聚合各子服务的入口。
主要目录/文件：
src/handlers/：主要业务处理器。
src/middleware/：中间件。
result.rs：统一返回结构。
1.6 app_socket
作用：WebSocket 服务，处理实时消息推送、连接管理等。
主要目录/文件：
src/handler/：各类 socket 事件处理器。
src/kafka/：Kafka 消息队列相关。
src/manager/：Socket 管理相关。
1.7 biz_service
作用：核心业务逻辑层，提供各类业务服务。
主要目录/文件：
proto/：所有业务相关的 proto 定义。
src/biz_service/：各类业务服务实现（用户、好友、群组、缓存、MQ等）。
src/entitys/：数据库实体定义。
src/manager/：业务管理器。
src/protocol/：协议转换、数据结构定义。
1.8 common
作用：通用库，提供配置、数据库、Redis、工具方法等。
主要目录/文件：
src/config/：配置管理。
src/db.rs：数据库相关。
src/redis/：Redis 操作封装。
src/util/：通用工具函数。
1.9 macro
作用：Rust 宏定义，便于代码复用和简化。
2. 技术栈与架构特点
语言：Rust
通信协议：gRPC（proto 文件）、RESTful API、WebSocket
消息队列：Kafka（用于消息推送、异步处理等）
数据库/缓存：有 Redis 操作相关代码，数据库部分未详细列出，但有实体定义
分布式/高可用：有分片管理、仲裁服务，适合分布式部署
文档：Swagger UI 支持
3. 典型业务流程（推测）
用户通过 API 网关（app_api）发起请求，如登录、发消息、加好友、建群等。
API 服务调用 biz_service，执行业务逻辑（如数据库操作、消息入队等）。
消息类操作通过 Kafka 投递到 app_socket，由 socket 服务推送到在线用户。
群组、好友等操作由 app_group、app_arb 等子服务处理，实现解耦和高可用。
所有服务共享 common 库，实现配置、数据库、缓存等基础功能。
4. 适用场景
大型 IM 系统后端
分布式、高并发场景
需要多端接入（Web、App、PC）的实时通讯
