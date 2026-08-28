<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# Ecat

[English](README.en.md) | [日本語](docs/i18n/ja/README.md) | [한국어](docs/i18n/ko/README.md) | [Русский](docs/i18n/ru/README.md) | [Deutsch](docs/i18n/de/README.md) | [Français](docs/i18n/fr/README.md) | [Español](docs/i18n/es/README.md) | [Português](docs/i18n/pt/README.md) | [हिन्दी](docs/i18n/hi/README.md) | [العربية](docs/i18n/ar/README.md) | [বাংলা](docs/i18n/bn/README.md) | [Bahasa Indonesia](docs/i18n/id/README.md) | 简体中文

Ecat中文名：一只猫

**一只猫** 是对标 [go-kratos/kratos](https://github.com/go-kratos/kratos) v3 的 Rust 微服务框架（v3.0.2 · 51 crates）。

提供 API-first 开发体验、可插拔的组件架构、统一的 HTTP/gRPC 中间件抽象，以及完备的 CLI 工具链。让熟悉 Kratos 的开发者可以无缝上手，同时充分利用 Rust 的类型安全、零成本抽象和极致性能。

<p align="center">
  <img src="docs/e-cat.svg" alt="Ecat 项目宠物（动态）" width="220" />
</p>

## 设计架构

```
┌──────────────────────────────────────────────────────────────┐
│                         ecat-cli                             │
│        (new │ proto │ run --watch │ build │ upgrade)         │
├──────────────────────────────────────────────────────────────┤
│                     ecat (应用生命周期)                         │
│      AppBuilder → App { name, servers, hooks, ... }         │
├────────────────────┬────────────────────┬────────────────────┤
│     transport      │    middleware      │     registry       │
│     ─────────      │    ──────────      │     ────────       │
│     HTTP (axum)    │    RecoveryLayer   │     memory         │
│     gRPC (tonic)   │    TracingLayer    │     consul         │
│     encoding       │    LoggingLayer    │                    │
│                    │    TimeoutLayer    │                    │
│                    │    RateLimitLayer  │                    │
│                    │    SecurityLayer   │                    │
│                    │    CircuitBreaker  │                    │
│                    │    Auth (JWT/API)  │                    │
├────────────────────┼────────────────────┼────────────────────┤
│     config         │     errors         │     metadata       │
│     ──────         │     ──────         │     ────────       │
│     file / env     │     ErrorCode      │     key-value      │
│     remote source  │     Error          │     HTTP/gRPC      │
├────────────────────┴────────────────────┴────────────────────┤
│                         data 层                               │
│     ────────────────────────────────────────────────          │
│     rdbms:   SQLite / PostgreSQL / MySQL / TiDB              │
│     cache:   Redis ✓                                         │
│     config:  remote (Consul KV)                              │
│     registry: consul                                         │
├──────────────────────────────────────────────────────────────┤
│                       ecat-protos                             │
│     (共享 .proto 定义: errors, metadata, ...)                 │
└──────────────────────────────────────────────────────────────┘
```

### 请求处理流程

```
客户端请求
  │
  ├─ HTTP 0.0.0.0:8000 ──→ axum::Router ──┐
  │                                        │
  └─ gRPC 0.0.0.0:9000 ──→ tonic::Server ─┤
                                      │
                              ┌───────┴───────┐
                              │   Middleware   │
                              │   ──────────   │
                              │ 1. Recovery    │  捕获 panic
                              │ 2. Tracing     │  注入 trace_id
                              │ 3. Logging     │  请求日志
                              │ 4. Auth        │  认证鉴权
                              │ 5. Metrics     │  指标采集
│ 6. Security    │  攻击检测
│ 7. CircuitBrk  │  熔断保护
                              └───────┬───────┘
                                      │
                              ┌───────┴───────┐
                              │    Handler     │  用户业务逻辑
                              │ (tower::Service)│
                              └───────┬───────┘
                                      │
                              ┌───────┴───────┐
                              │   Response     │  编码序列化
                              │ JSON/Protobuf  │
                              └───────────────┘
```

## 功能

- **API-first**：Protobuf 定义 API、错误码、元数据；prost + tonic-build 代码生成
- **双协议支持**：HTTP（axum）和 gRPC（tonic）共用同一套 tower::Layer 中间件
- **可插拔架构**：Registry、Config、Logging、Encoding 全部通过 trait 抽象，默认提供生产可用实现
- **中间件体系**：内置 Recovery、Tracing、Logging、Timeout、RateLimit、Security、CircuitBreaker、MetricsLayer、RetryLayer、ValidateLayer、CORS（cors feature）；通过 tower::ServiceBuilder 组合
- **应用生命周期**：Builder 模式构建 App，多 Server 并发启动，SIGTERM/SIGINT 信号处理，start/stop 生命周期钩子
- **类型安全**：基于 protobuf 的错误码体系，编译期 HTTP 状态码映射
- **可观测性**：tracing + Prometheus + Health 端点（/health、/ready）
- **攻击检测**：SecurityLayer 自动检测 SQL 注入、XSS、SSRF 等攻击模式，阻断高危请求
- **服务间通信**：HttpClient 集成服务发现与负载均衡，CircuitBreaker 熔断保护
- **认证鉴权**：JWT / API Key 认证中间件，Claims 传递至请求上下文
- **消息与事件**：MessageQueue trait + EventBus 本地/远程 Pub/Sub
- **分布式追踪**：请求 span、trace_id 注入/提取
- **gRPC 客户端**：GrpcClient 集成服务发现与负载均衡
- **多协议**：HTTP、gRPC、WebSocket、GraphQL 统一路由
- **多数据源**：RDBMS（SQLite/PG/MySQL/TiDB）、缓存（Redis/Memcached）、搜索（OpenSearch/Elasticsearch）、图（Neo4j/NebulaGraph/ArangoDB）、时序（InfluxDB/IoTDB/QuestDB/TDengine）、文档（MongoDB）、对象存储（S3/MinIO）

### Kratos 概念映射

| Kratos (Go) | e-cat (Rust) | 说明 |
|-------------|-------------|------|
| `kratos.New()` | `App::builder()` | Builder 模式 |
| `http.Handler` | `tower::Service` | Rust 生态标准 trait |
| `http.Server` | `axum::Router` | 社区主流 HTTP 框架 |
| `grpc.Server` | `tonic::transport::Server` | 最成熟的 gRPC 实现 |
| `proto generate` | `prost + tonic-build` | 社区标准 protobuf |
| `registry.Discovery` | `Registry` trait | 可插拔注册发现 |
| `config.Source` | `ConfigSource` trait | 多源配置加载 |

## 技术栈

| 组件 | 选型 |
|------|------|
| 异步运行时 | **tokio** |
| HTTP | **axum** |
| gRPC | **tonic** |
| Protobuf | **prost + tonic-build** |
| 中间件 | **tower::Service / Layer** |
| 日志/追踪 | **tracing + trace_id propagation** |
| 指标 | **prometheus** |
| 序列化 | **serde + prost** |
| 攻击检测 | **security-rust** |
| RDBMS | **sqlx** |
| Redis | **redis-rs** |
| JWT | **jsonwebtoken** |
| HTTP Client | **reqwest** |
| CLI | **clap** |

## 支持的数据库

| 类别 | 数据库 | Crate | 状态 |
|------|--------|-------|------|
| RDBMS | SQLite | `ecat-data-sqlx` | ✅ 已实现 |
| RDBMS | PostgreSQL | `ecat-data-sqlx` | ✅ 已实现 |
| RDBMS | MySQL | `ecat-data-sqlx` | ✅ 已实现 |
| RDBMS | TiDB | `ecat-data-sqlx` | ✅ 已实现 |
| 缓存 | Redis | `ecat-data-redis` | ✅ 已实现 |
| 搜索 | OpenSearch | `ecat-data-opensearch` | ✅ 已实现 |
| 搜索 | Elasticsearch | `ecat-data-elasticsearch` | ✅ 已实现 |
| 缓存 | Memcached | `ecat-data-memcached` | ⚠️ 内存实现（非生产，勿用于持久缓存） |
| OLAP | ClickHouse | `ecat-data-clickhouse` | ✅ 已实现 |
| 图 | Neo4j | `ecat-data-neo4j` | ✅ REST API |
| 图 | NebulaGraph | `ecat-data-nebulagraph` | ✅ REST API |
| 图 | ArangoDB | `ecat-data-arangodb` | ✅ REST API |
| 时序 | InfluxDB | `ecat-data-influxdb` | ✅ HTTP API |
| 时序 | Apache IoTDB | `ecat-data-iotdb` | ✅ REST API |
| 时序 | QuestDB | `ecat-data-questdb` | ✅ HTTP API |
| 时序 | TDengine | `ecat-data-tdengine` | ✅ REST API |
| 文档 | MongoDB | `ecat-data-mongodb` | ✅ 原生驱动 |
| 对象存储 | S3 / MinIO | `ecat-data-s3` | ✅ reqwest+rustls |

> 所有数据后端通过统一的 trait 抽象（`RdbmsClient` / `Cache` / `SearchClient` / `GraphClient` / `TsdbClient` / `DocumentClient` / `StorageClient`），按需引入对应 contrib crate。每个后端均提供 `XxxConfig` 结构体（`#[derive(Deserialize)]`），支持从 JSON/YAML 配置文件加载连接信息。

> **构造器命名约定**：消息队列 crate（`ecat-mq-*`）主构造器统一为 `connect`（如 `KafkaMq::connect(brokers)`、`MqttMq::connect(url)`），另提供 `from_config` 从配置加载；数据后端 crate（`ecat-data-*`）多数主构造器为 `new`，例外：`ecat-data-redis` / `ecat-data-sqlx` 沿用 `connect`，`ecat-data-mongodb` / `ecat-data-s3` 仅提供 `from_config`。此为既有约定，不强制统一（避免破坏性变更）；3.0 窗口可评估统一。

### 数据库配置示例

每个数据后端提供 `XxxConfig` 结构体和 `from_config()` 方法，将连接信息从代码中解耦到配置文件：

```rust
use ecat_data_redis::{RedisCache, RedisConfig};
use ecat_data_sqlx::{SqlxClient, SqlxConfig};
use ecat_data_clickhouse::{ClickhouseClient, ClickhouseConfig};

// 从配置文件加载（JSON 或 YAML）
let config: serde_json::Value = serde_json::from_str(r#"{
    "redis":     {"url": "redis://localhost:6379"},
    "sql":       {"url": "postgres://user:pass@localhost/db"},
    "clickhouse":{"base_url": "http://localhost:8123", "database": "mydb"}
}"#)?;

// Redis
let redis_cfg: RedisConfig = serde_json::from_value(config["redis"].clone())?;
let cache = RedisCache::from_config(redis_cfg).await?;
cache.set("key", b"value", Duration::from_secs(60)).await?;

// RDBMS
let sql_cfg: SqlxConfig = serde_json::from_value(config["sql"].clone())?;
let db = SqlxClient::from_config(sql_cfg).await?;
let rows = db.query("SELECT * FROM users").await?;

// ClickHouse
let ch_cfg: ClickhouseConfig = serde_json::from_value(config["clickhouse"].clone())?;
let ch = ClickhouseClient::from_config(ch_cfg);
ch.execute("INSERT INTO events VALUES (1, 'start')").await?;
```

**配置字段参考**:

| 后端 | Config | 字段 | 示例值 |
|------|--------|------|--------|
| Redis | `RedisConfig` | `url`, `password`? | `redis://localhost:6379` |
| RDBMS | `SqlxConfig` | `url`, `username`?, `password`? | `postgres://localhost/db` |
| ClickHouse | `ClickhouseConfig` | `base_url`, `database`, `username`?, `password`? | `http://localhost:8123`, `default` |
| QuestDB | `QuestdbConfig` | `base_url`, `username`?, `password`? | `http://localhost:9000` |
| Elasticsearch | `ElasticsearchConfig` | `base_url`, `username`?, `password`? | `http://localhost:9200` |
| OpenSearch | `OpenSearchConfig` | `base_url`, `username`?, `password`? | `http://localhost:9200` |
| InfluxDB | `InfluxConfig` | `base_url`, `org`, `bucket`, `token` | — |
| Neo4j | `Neo4jConfig` | `base_url`, `username`, `password` | — |
| NebulaGraph | `NebulaGraphConfig` | `base_url`, `space`, `username`?, `password`? | — |
| ArangoDB | `ArangoConfig` | `base_url`, `db`, `username`, `password` | — |
| IoTDB | `IotdbConfig` | `base_url`, `username`, `password` | — |
| Memcached | `MemcachedConfig` | `username`?, `password`?（保留字段） | — |
| TDengine | `TdengineConfig` | `base_url`, `username`, `password`, `database`? | `http://localhost:6041` |
| MongoDB | `MongoConfig` | `url`, `database`, `tls`? | `mongodb://localhost:27017`, `app` |
| S3 | `S3Config` | `endpoint`, `region`, `access_key`, `secret_key`, `tls`? | `http://localhost:9000`, `us-east-1` |

> 所有后端 Config 均支持可选的 `tls` 字段（`TlsClientConfig`），用于配置 TLS 客户端证书认证。详见 [数据库配置教程](docs/database-config-tutorial.md)。

## 项目结构

```
e-cat/
├── ecat/                       # 核心：App 生命周期
├── ecat-transport/             # 传输抽象（Server trait）
├── ecat-transport-http/        # axum 实现
├── ecat-transport-grpc/        # tonic 实现
├── ecat-middleware/            # tower::Layer 中间件
├── ecat-protos/                # Protobuf 定义
├── ecat-errors/                # 错误码体系
├── ecat-metadata/              # 元数据传递
├── ecat-encoding/              # 序列化抽象
├── ecat-logging/               # tracing 集成
├── ecat-registry/              # 服务注册发现
├── ecat-config/                # 配置管理
├── ecat-metrics/               # Prometheus 集成
├── ecat-data/                  # 数据访问 trait
├── ecat-security/              # 攻击检测（security-rust）
├── ecat-cli/                   # CLI 工具
├── ecat-health/                # 健康检查（/health /ready）
├── ecat-auth/                  # 认证中间件（JWT / API Key）
├── ecat-client/                # 服务间 HTTP 客户端
├── ecat-circuit-breaker/       # 熔断器（Tower Layer）
├── ecat-registry-consul/       # Consul 服务注册
├── ecat-config-remote/         # Consul KV 远程配置
├── ecat-data-redis/            # Redis 缓存实现
├── ecat-mq/                    # 消息队列抽象
├── ecat-events/                # 事件总线（本地 + 远程）
├── ecat-testing/               # 集成测试工具
├── ecat-openapi/               # OpenAPI spec 生成
├── ecat-bench/                 # 性能基准
├── ecat-tracing/               # 分布式追踪（trace_id 注入/提取）
├── ecat-registry-etcd/         # etcd 服务注册
├── ecat-mq-kafka/              # Kafka 消息队列适配
├── ecat-data-opensearch/       # OpenSearch 搜索后端
├── ecat-data-influxdb/         # InfluxDB 时序后端
├── ecat-graphql/               # GraphQL endpoint
├── ecat-data-elasticsearch/    # Elasticsearch 搜索后端
├── ecat-data-clickhouse/       # ClickHouse OLAP 后端
├── ecat-data-sqlx/             # RDBMS 后端（SQLite/PG/MySQL/TiDB）
├── ecat-data-memcached/        # Memcached 缓存后端（内存实现）
├── ecat-data-neo4j/            # Neo4j 图后端
├── ecat-data-nebulagraph/      # NebulaGraph 图后端
├── ecat-data-arangodb/         # ArangoDB 图后端
├── ecat-data-iotdb/            # IoTDB 时序后端
├── ecat-data-questdb/          # QuestDB 时序后端
├── ecat-transport-ws/          # WebSocket transport
├── ecat-versioning/            # API 版本路由
├── ecat-tls/                   # TLS 证书配置与自动生成
├── ecat-deploy/                # Docker / K8s / Helm / CI/CD
├── ecat-lock/                  # 分布式锁抽象（Redis 实现）
├── ecat-scheduler/             # tokio 定时任务调度
├── ecat-tracing-otlp/          # OpenTelemetry OTLP 追踪导出
├── ecat-data-tdengine/         # TDengine 时序后端
├── ecat-data-mongodb/          # MongoDB 文档后端
├── ecat-data-s3/               # S3 / MinIO 对象存储后端
├── ecat-mq-rabbitmq/           # RabbitMQ 消息后端
├── ecat-mq-mqtt/               # MQTT 消息后端
├── ecat-mq-nats/               # NATS 消息后端
├── config/                     # 配置示例文件
├── docs/                       # 设计文档与生态规划
└── examples/                   # 示例项目
```

## 快速开始

### 前提条件

- Rust 1.85+（stable 工具链，edition 2024 要求）
- [protoc](https://github.com/protocolbuffers/protobuf)（Protocol Buffers 编译器）

### 安装 CLI

```bash
cargo install ecat-cli
```

### 创建服务

```bash
# 脚手架生成项目
ecat new helloworld
cd helloworld

# 添加 proto 定义
ecat proto add proto/service.proto

# 生成客户端和服务端代码（tonic-build build.rs，自动补齐 Cargo.toml 依赖）
ecat proto client proto/service.proto
ecat proto server proto/service.proto -t internal/service

# 开发模式运行
ecat run

# 监听 src/ 变更自动重启
ecat run --watch

# 更新所有 ecat-* 依赖
ecat upgrade
```

访问 `http://localhost:8000/helloworld/ecat`。

### 代码示例

```rust
use ecat::App;
use ecat_transport_http::HttpServer;
use ecat_transport_grpc::GrpcServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let http_srv = HttpServer::new("0.0.0.0:8000");
    let grpc_srv = GrpcServer::new("0.0.0.0:9000");

    let app = App::builder()
        .name("my-service")
        .version("v1.0.0")
        .server(http_srv)
        .server(grpc_srv)
        .on_start(|| async {
            tracing::info!("service started");
            Ok(())
        })
        .on_stop(|| async {
            tracing::info!("service stopped");
            Ok(())
        })
        .build()?;

    app.run().await?; // 阻塞直到 SIGTERM/SIGINT
    Ok(())
}
```

### 聚合 crate（ecat）

`ecat` 提供 feature-gated 的 re-export 入口——只启用需要的组件：

```rust
use ecat::transport_http::HttpServer;   // feature "http"（默认）
use ecat::middleware::RecoveryLayer;     // feature "middleware"
use ecat::auth::JwtAuthLayer;            // feature "auth"
use ecat::data::redis::RedisCache;       // feature "redis"
```

默认 features = `http+grpc`；使用 `--no-default-features --features <组件>` 可精简依赖树。完整 feature 列表：`http` `grpc` `middleware` `auth` `client` `events` `metrics` `tracing` `circuit-breaker` `consul` `remote` `redis`。

### 中间件

```rust
use tower::ServiceBuilder;
use ecat_middleware::{RecoveryLayer, TracingLayer, LoggingLayer, TimeoutLayer};
use ecat_circuit_breaker::CircuitBreakerLayer;
use ecat_security::SecurityLayer;
use ecat_auth::JwtAuthLayer;
use std::time::Duration;

// JWT 密钥需 ≥32 字节；可链式强制校验 iss/aud 声明（可选，默认不校验）：
// JwtAuthLayer::new(secret)?.required_issuer("my-issuer").required_audience("my-api")
let jwt = JwtAuthLayer::new("change-me-32-bytes-minimum-secret").expect("valid jwt secret");

let layer = ServiceBuilder::new()
    .layer(RecoveryLayer)
    .layer(TracingLayer)
    .layer(LoggingLayer)
    .layer(TimeoutLayer::new(Duration::from_secs(30)))
    .layer(CircuitBreakerLayer::new())
    .layer(jwt)
    .layer(SecurityLayer::new());
```

> 注：`ecat_middleware::TracingLayer` 不注入 trace_id；如需请求级 trace_id 注入，请使用 `ecat_tracing::TracingLayer::new()`。

```rust
// 指标：记录请求计数与时延到全局 registry（与 /metrics 端点共享）
use ecat_metrics::MetricsLayer;
let app = Router::new().route("/hello", get(hello)).layer(MetricsLayer::new());
// 指标名：ecat_http_requests_total / ecat_http_request_duration_seconds
// （标签 method/path/status）。路径含 ID 等高基数场景请用
// MetricsLayer::new().with_path_fn(...) 归一化，避免指标基数爆炸。

// 重试：指数退避；⚠️ 仅对幂等请求（GET/HEAD/PUT/DELETE）安全
use ecat_middleware::RetryLayer;
let retry = RetryLayer::new(3, Duration::from_secs(1), Duration::from_secs(30)); // 含首次共 3 次尝试
// 自定义重试规则：RetryLayer::new(3, ...).with_rule(MyRule)  // 按状态码/响应内容判定

// 校验：路由前校验 header/参数，失败短路返回 JSON 错误（默认 400，with_status 可改 422 等）
use ecat_middleware::{ValidateLayer, ValidateError};
let validate = ValidateLayer::from_fn(|req: &http::Request<axum::body::Body>| {
    if req.headers().contains_key("x-api-key") {
        Ok(())
    } else {
        Err(ValidateError::new("missing x-api-key").with_status(422))
    }
});

// CORS：ecat-middleware 需启用 "cors" feature
use ecat_middleware::{CorsLayer, AllowOrigin};
let cors = CorsLayer::new().allow_origin(AllowOrigin::any());
```

### 错误处理

```rust
use ecat_errors::{Error, ErrorCode};

fn get_user(id: u64) -> Result<User, Error> {
    if id == 0 {
        return Err(Error::new(
            ErrorCode::InvalidArgument,
            "bad_request",
            "user id must be positive",
        ));
    }
    // ...
}
```

## 实现阶段

| 阶段 | 状态 | 内容 |
|------|------|------|
| Phase 1 | ✅ 完成 | 项目骨架、protos、errors、metadata、encoding、logging |
| Phase 2 | ✅ 完成 | Transport 层（HTTP + gRPC） |
| Phase 3 | ✅ 完成 | Middleware 体系（Recovery/Tracing/Logging/Timeout） |
| Phase 4 | ✅ 完成 | App 生命周期管理 |
| Phase 5 | ✅ 完成 | Registry、Config、Metrics |
| Phase 5.5 | ✅ 完成 | Data 访问层（traits + sqlx 后端） |
| Phase 6 | ✅ 完成 | CLI 工具链（new/proto/run/build） |
| Phase 7 | ✅ 完成 | README、示例（helloworld）、设计文档 |
| Phase 8 | ✅ 完成 | 攻击检测集成（security-rust, ecat-security） |
| Phase 9 | ✅ 完成 | 生态一期（health / client / circuit-breaker / auth / registry-consul） |
| Phase 10 | ✅ 完成 | 生态二期（redis / mq / events / config-remote） |
| Phase 11 | ✅ 完成 | 生态三期（testing / deploy / bench / openapi） |
| Phase 12 | ✅ 完成 | 通信与安全强化（gRPC 客户端 / OAuth2 / mTLS / 分布式追踪） |
| Phase 13 | ✅ 完成 | 数据后端补齐（etcd / Kafka / OpenSearch / InfluxDB） |
| Phase 14 | ✅ 完成 | 运维与体验（WebSocket / API 版本管理 / Helm / CI/CD） |
| Phase 15 | ✅ 完成 | 生态扩展 v2（真 Kafka / RabbitMQ / MQTT / NATS / MongoDB / S3 / TDengine / OTLP / 分布式锁 / 调度 / CLI watch+upgrade） |
| Phase 16 | ✅ 完成 | 维护强化 v2.4（M1 MetricsLayer / M2 RetryLayer / M3 ValidateLayer / M4 CORS / U1 聚合 crate ecat / U2 examples / OAuth2 token hash / CVE 跟踪） |

## 已知限制

- **GraphQL 解析（ecat-graphql）**：支持字段参数与嵌套 selection（`query_field`/`mutation_field` 富 resolver 可访问 `args`/`variables`/`selection`）；仍不支持别名、fragment 与多顶层字段，请勿将其暴露为通用 GraphQL 端点。
- **OAuth2 内省缓存（ecat-auth）**：缓存 key 为 token 的 SHA-256 hash（不存 token 明文）；缓存值经白名单过滤（默认保留 sub/exp/iat/role + extra 的 iss/aud/scope/roles，`cache_claims_whitelist` 可配置；miss 时仍返回完整 claims，仅缓存值过滤）；TTL 过期条目在写入时主动清除（默认 TTL 300s）。
- **Kafka offset（ecat-mq-kafka）**：默认 `enable.auto.commit=false` 且无手动 commit——进程重启后从分区末尾（latest）重读，停机期间产生的消息会被跳过；需显式配置 `auto_commit=true` 才具备 at-least-once 语义（重启从最近提交点继续）。

## 设计目标

| # | 目标 | 说明 |
|---|------|------|
| 1 | **Kratos 对齐** | 保持 Kratos 的 API-first、可插拔、统一抽象理念 |
| 2 | **Rust 惯用** | 复用 tower::Service、trait 泛型、零成本抽象；不做「Go in Rust」 |
| 3 | **类型安全** | 编译期捕获错误，Protobuf 定义全强类型化 |
| 4 | **可插拔** | Registry、Config、Logging、Encoding 全部通过 trait 抽象 |
| 5 | **工具链完备** | CLI 支持项目脚手架、proto 代码生成、开发运行 |
| 6 | **性能优先** | 零成本抽象 + 异步运行时 |
| 7 | **可观测** | tracing + Prometheus 开箱即用 |
| 8 | **生态完备** | 客户端、熔断、认证、健康检查、注册中心后端 |

## 技术说明

### 为什么选择 tower::Service

[`tower::Service`](https://docs.rs/tower/latest/tower/trait.Service.html) 是 Rust 异步生态的 `http.Handler` 等价物。axum 和 tonic 都构建在 tower 之上，因此 e-cat 不需要自定义中间件 trait——直接提供 tower::Layer 实现即可达到与 Kratos 中间件相同的效果，且零适配器开销。

### 为什么用 Cargo Workspace

与 Kratos 的模块化设计一致。所有 `ecat-*` crate 以 workspace 锁步版本发布（当前 3.0.2），各自独立编译，用户按需引入。核心 crate 保持最小依赖，contrib crate 提供可选集成。

### 为什么用 prost（而非 protobuf-rs）

prost 是 Rust 社区最广泛使用的 protobuf 实现，编译期生成类型安全代码，与 tonic 深度集成。

## 设计文档

- [设计规范](docs/superpowers/specs/2026-07-29-ecat-framework-design.md)
- [实现计划](docs/superpowers/plans/2026-07-29-ecat-framework.md)
- [生态规划 v1](docs/ecosystem-plan.md)（已完成）
- [生态规划 v2](docs/ecosystem-plan-v2.md)（已完成）
- [生态规划 v3](docs/ecosystem-plan-v3.md)（最终评估）
- [API 参考](docs/api.md)
- [审计报告 r5](docs/audit-report-2026-08-01-r5.md)（2026-08-01）
- [数据库配置教程](docs/database-config-tutorial.md)
- [依赖 CVE 跟踪](docs/dependency-cve-tracking.md)
- [TLS 证书认证教程](docs/tls-certificate-tutorial.md)
- [配置示例文件](config/databases.example.yaml)

## 支持

欢迎支持本项目！

| 微信支付 | 支付宝 |
|:---:|:---:|
| <img src="docs/weixinpay.png" width="130" height="130" alt="微信支付"> | <img src="docs/alipay.png" width="130" height="130" alt="支付宝"> |

### 全球转账（银行汇款）

| 项目 | 信息 |
|------|------|
| 收款人姓名 | WANG KEXUN |
| 收款账户号码 | 881015918251 |
| 收款银行 | ZA Bank Limited |
| SWIFT Code | AABLHKHHXXX |
| 银行编号 | 387 |
| 银行地址 | Core F, Cyberport 3, 100 Cyberport Road, Hong Kong |

> **跨境汇款代理银行（如需）**：此为代理银行（中转银行）信息，非收款银行信息，请向汇款银行查询是否需要提供。
>
> - 汇入港元、人民币及美元：**Citibank N.A. Hong Kong**（SWIFT：`CITIHKHXXXX`，银行编号：006，分行：Hong Kong Branch，分行编号：391，地址：Citibank Tower, Citibank Plaza, 3 Garden Road, Central, Hong Kong）
> - 汇入其他币种：**THE BANK OF NEW YORK MELLON**（SWIFT：`IRVTUS3NXXX`，地址：240 GREENWICH STREET, NEW YORK, United States）

## 许可证

Apache-2.0
