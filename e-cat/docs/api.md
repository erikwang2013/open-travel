<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# Ecat API 参考

本页汇总 Ecat 框架的接口（API）面：端口约定、内置端点、错误格式与扩展接口。业务路由由各服务自行注册。

## 端口约定

| 协议 | 监听地址 | 说明 |
|------|----------|------|
| HTTP | `0.0.0.0:8000` | axum 路由，默认示例端口 |
| gRPC | `0.0.0.0:9000` | tonic Server，默认示例端口 |

## 内置端点

以下端点由生态 crate 提供，随服务挂载：

| 端点 | 来源 | 说明 |
|------|------|------|
| `/health` | ecat-health | 存活检查（返回服务名、版本、启动时间） |
| `/ready` | ecat-health | 就绪检查（依赖就绪后返回 200） |
| `/metrics` | ecat-metrics | Prometheus 指标暴露（`ecat_http_requests_total` / `ecat_http_request_duration_seconds`） |
| `/{service}/{method}` | 用户路由 | 示例：`/helloworld/ecat` |

> 指标端点路径含 ID 等高基数场景请用 `MetricsLayer::new().with_path_fn(...)` 归一化，避免指标基数爆炸。

## 请求处理流程

```
客户端请求
  ├─ HTTP :8000 ──→ axum::Router ─┐
  └─ gRPC :9000 ──→ tonic::Server ─┤
                              ┌─────┴──────┐
                              │ Middleware │  Recovery→Tracing→Logging→Auth→Metrics→Security→CircuitBreaker
                              └─────┬──────┘
                                    ▼
                               Handler（tower::Service）
                                    ▼
                               Response（JSON/Protobuf 编码）
```

## 错误格式

`ecat-errors` 提供 `ErrorCode` + `Error`，编译期映射 HTTP 状态码：

```rust
use ecat_errors::{Error, ErrorCode};

Error::new(ErrorCode::InvalidArgument, "bad_request", "user id must be positive");
```

错误响应经 middleware 编码为 JSON（或 Protobuf），携带 code / reason / message。

## 扩展接口

| 能力 | Crate | 接口 |
|------|-------|------|
| GraphQL | ecat-graphql | `/graphql` 端点；支持字段参数与嵌套 selection，不支持别名、fragment 与多顶层字段 |
| OpenAPI | ecat-openapi | 从路由生成 OpenAPI spec |
| WebSocket | ecat-transport-ws | 升级的 WS 传输 |
| API 版本路由 | ecat-versioning | `/v1/...` 前缀版本路由 |
| 认证 | ecat-auth | JWT / API Key 中间件；JWT 密钥需 ≥32 字节，可链式 `required_issuer`/`required_audience` |
| gRPC 客户端 | ecat-transport-grpc | 集成服务发现与负载均衡 |

## 服务间通信

- `HttpClient`（ecat-client）：集成服务发现与负载均衡，CircuitBreaker 熔断保护
- `GrpcClient`（ecat-transport-grpc）：同上，gRPC 协议
- 中间件统一使用 `tower::ServiceBuilder` 组合（Recovery / Tracing / Logging / Timeout / RateLimit / Security / CircuitBreaker / Metrics / Retry / Validate / CORS）

## 数据后端接口

所有数据后端（`ecat-data-*`）通过统一 trait（`RdbmsClient` / `Cache` / `SearchClient` / `GraphClient` / `TsdbClient` / `DocumentClient` / `StorageClient`）抽象；REST 类后端（Neo4j / NebulaGraph / ArangoDB / InfluxDB / IoTDB / QuestDB / TDengine / OpenSearch / Elasticsearch / S3）基于 `base_url` 访问对应 HTTP 接口。连接配置见 [数据库配置教程](database-config-tutorial.md)。
