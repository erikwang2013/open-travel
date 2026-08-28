<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# Ecat

[简体中文](../../../README.md) | **[English](../../../README.en.md)** | [日本語](../ja/README.md) | [한국어](../ko/README.md) | [Русский](../ru/README.md) | [Deutsch](../de/README.md) | [Français](../fr/README.md) | [Español](../es/README.md) | [Português](../pt/README.md) | [हिन्दी](../hi/README.md) | [العربية](../ar/README.md) | [বাংলা](../bn/README.md) | [Bahasa Indonesia](../id/README.md)

Ecat's Chinese name: 一只猫 (literally "a cat")

**一只猫** is a Rust microservice framework benchmarked against [go-kratos/kratos](https://github.com/go-kratos/kratos) v3 (v3.0.2 · 51 crates).

It offers an API-first development experience, a pluggable component architecture, a unified HTTP/gRPC middleware abstraction, and a complete CLI toolchain. Developers familiar with Kratos can get started seamlessly, while fully leveraging Rust's type safety, zero-cost abstractions, and extreme performance.

<p align="center">
  <img src="e-cat.svg" alt="Ecat project mascot (animated)" width="220" />
</p>

## Design Architecture

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

### Request Handling Flow

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

## Features

- **API-first**: define APIs, error codes, and metadata in Protobuf; code generation via prost + tonic-build
- **Dual protocol support**: HTTP (axum) and gRPC (tonic) share the same set of tower::Layer middleware
- **Pluggable architecture**: Registry, Config, Logging, and Encoding are all abstracted through traits, with production-ready default implementations
- **Middleware system**: built-in Recovery, Tracing, Logging, Timeout, RateLimit, Security, CircuitBreaker, MetricsLayer, RetryLayer, ValidateLayer, CORS (cors feature); composed via tower::ServiceBuilder
- **Application lifecycle**: Builder pattern to construct App, multiple servers started concurrently, SIGTERM/SIGINT signal handling, start/stop lifecycle hooks
- **Type safety**: protobuf-based error code system with compile-time HTTP status mapping
- **Observability**: tracing + Prometheus + Health endpoints (/health, /ready)
- **Attack detection**: SecurityLayer automatically detects attack patterns such as SQL injection, XSS, and SSRF, blocking high-risk requests
- **Inter-service communication**: HttpClient integrates service discovery and load balancing, with CircuitBreaker protection
- **Authentication and authorization**: JWT / API Key authentication middleware, Claims passed to the request context
- **Messaging and events**: MessageQueue trait + EventBus local/remote Pub/Sub
- **Distributed tracing**: request spans, trace_id injection/extraction
- **gRPC client**: GrpcClient integrates service discovery and load balancing
- **Multi-protocol**: unified routing for HTTP, gRPC, WebSocket, and GraphQL
- **Multiple data sources**: RDBMS (SQLite/PG/MySQL/TiDB), cache (Redis/Memcached), search (OpenSearch/Elasticsearch), graph (Neo4j/NebulaGraph/ArangoDB), time series (InfluxDB/IoTDB/QuestDB/TDengine), document (MongoDB), object storage (S3/MinIO)

### Kratos Concept Mapping

| Kratos (Go) | e-cat (Rust) | Notes |
|-------------|-------------|------|
| `kratos.New()` | `App::builder()` | Builder pattern |
| `http.Handler` | `tower::Service` | Standard trait in the Rust ecosystem |
| `http.Server` | `axum::Router` | Mainstream HTTP framework in the community |
| `grpc.Server` | `tonic::transport::Server` | The most mature gRPC implementation |
| `proto generate` | `prost + tonic-build` | Community-standard protobuf |
| `registry.Discovery` | `Registry` trait | Pluggable registry and discovery |
| `config.Source` | `ConfigSource` trait | Multi-source configuration loading |

## Tech Stack

| Component | Choice |
|------|------|
| Async runtime | **tokio** |
| HTTP | **axum** |
| gRPC | **tonic** |
| Protobuf | **prost + tonic-build** |
| Middleware | **tower::Service / Layer** |
| Logging/tracing | **tracing + trace_id propagation** |
| Metrics | **prometheus** |
| Serialization | **serde + prost** |
| Attack detection | **security-rust** |
| RDBMS | **sqlx** |
| Redis | **redis-rs** |
| JWT | **jsonwebtoken** |
| HTTP Client | **reqwest** |
| CLI | **clap** |

## Supported Databases

| Category | Database | Crate | Status |
|------|--------|-------|------|
| RDBMS | SQLite | `ecat-data-sqlx` | ✅ Implemented |
| RDBMS | PostgreSQL | `ecat-data-sqlx` | ✅ Implemented |
| RDBMS | MySQL | `ecat-data-sqlx` | ✅ Implemented |
| RDBMS | TiDB | `ecat-data-sqlx` | ✅ Implemented |
| Cache | Redis | `ecat-data-redis` | ✅ Implemented |
| Search | OpenSearch | `ecat-data-opensearch` | ✅ Implemented |
| Search | Elasticsearch | `ecat-data-elasticsearch` | ✅ Implemented |
| Cache | Memcached | `ecat-data-memcached` | ⚠️ In-memory implementation (not for production, do not use for persistent caching) |
| OLAP | ClickHouse | `ecat-data-clickhouse` | ✅ Implemented |
| Graph | Neo4j | `ecat-data-neo4j` | ✅ REST API |
| Graph | NebulaGraph | `ecat-data-nebulagraph` | ✅ REST API |
| Graph | ArangoDB | `ecat-data-arangodb` | ✅ REST API |
| Time series | InfluxDB | `ecat-data-influxdb` | ✅ HTTP API |
| Time series | Apache IoTDB | `ecat-data-iotdb` | ✅ REST API |
| Time series | QuestDB | `ecat-data-questdb` | ✅ HTTP API |
| Time series | TDengine | `ecat-data-tdengine` | ✅ REST API |
| Document | MongoDB | `ecat-data-mongodb` | ✅ Native driver |
| Object storage | S3 / MinIO | `ecat-data-s3` | ✅ reqwest+rustls |

> All data backends are abstracted through unified traits (`RdbmsClient` / `Cache` / `SearchClient` / `GraphClient` / `TsdbClient` / `DocumentClient` / `StorageClient`); import the corresponding contrib crate as needed. Each backend provides an `XxxConfig` struct (`#[derive(Deserialize)]`) that supports loading connection information from JSON/YAML config files.

> **Constructor naming convention**: the message queue crates (`ecat-mq-*`) uniformly use `connect` as the primary constructor (e.g. `KafkaMq::connect(brokers)`, `MqttMq::connect(url)`), and also provide `from_config` for loading from config; most data backend crates (`ecat-data-*`) use `new`, with exceptions: `ecat-data-redis` / `ecat-data-sqlx` keep `connect`, and `ecat-data-mongodb` / `ecat-data-s3` only provide `from_config`. This is an existing convention and is not forced to be unified (to avoid breaking changes); unification can be evaluated in the 3.0 window.

### Database Configuration Example

Each data backend provides an `XxxConfig` struct and a `from_config()` method that decouples connection information from code into config files:

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

**Config field reference**:

| Backend | Config | Fields | Example values |
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
| Memcached | `MemcachedConfig` | `username`?, `password`? (reserved fields) | — |
| TDengine | `TdengineConfig` | `base_url`, `username`, `password`, `database`? | `http://localhost:6041` |
| MongoDB | `MongoConfig` | `url`, `database`, `tls`? | `mongodb://localhost:27017`, `app` |
| S3 | `S3Config` | `endpoint`, `region`, `access_key`, `secret_key`, `tls`? | `http://localhost:9000`, `us-east-1` |

> All backend Configs support an optional `tls` field (`TlsClientConfig`) for configuring TLS client certificate authentication. See [Database Configuration Tutorial](database-config-tutorial.md).

## Project Structure

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

## Quick Start

### Prerequisites

- Rust 1.85+ (stable toolchain, edition 2024 required)
- [protoc](https://github.com/protocolbuffers/protobuf) (Protocol Buffers compiler)

### Install the CLI

```bash
cargo install ecat-cli
```

### Create a Service

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

Visit `http://localhost:8000/helloworld/ecat`.

### Code Example

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

### The Umbrella Crate (ecat)

`ecat` provides a feature-gated re-export entry point — enable only the components you need:

```rust
use ecat::transport_http::HttpServer;   // feature "http"（默认）
use ecat::middleware::RecoveryLayer;     // feature "middleware"
use ecat::auth::JwtAuthLayer;            // feature "auth"
use ecat::data::redis::RedisCache;       // feature "redis"
```

Default features = `http+grpc`; use `--no-default-features --features <component>` to slim down the dependency tree. Full feature list: `http` `grpc` `middleware` `auth` `client` `events` `metrics` `tracing` `circuit-breaker` `consul` `remote` `redis`.

### Middleware

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

> Note: `ecat_middleware::TracingLayer` does not inject trace_id; for request-level trace_id injection, use `ecat_tracing::TracingLayer::new()`.

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

### Error Handling

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

## Implementation Phases

| Phase | Status | Content |
|------|------|------|
| Phase 1 | ✅ Done | Project skeleton, protos, errors, metadata, encoding, logging |
| Phase 2 | ✅ Done | Transport layer (HTTP + gRPC) |
| Phase 3 | ✅ Done | Middleware system (Recovery/Tracing/Logging/Timeout) |
| Phase 4 | ✅ Done | App lifecycle management |
| Phase 5 | ✅ Done | Registry, Config, Metrics |
| Phase 5.5 | ✅ Done | Data access layer (traits + sqlx backend) |
| Phase 6 | ✅ Done | CLI toolchain (new/proto/run/build) |
| Phase 7 | ✅ Done | README, examples (helloworld), design docs |
| Phase 8 | ✅ Done | Attack detection integration (security-rust, ecat-security) |
| Phase 9 | ✅ Done | Ecosystem phase 1 (health / client / circuit-breaker / auth / registry-consul) |
| Phase 10 | ✅ Done | Ecosystem phase 2 (redis / mq / events / config-remote) |
| Phase 11 | ✅ Done | Ecosystem phase 3 (testing / deploy / bench / openapi) |
| Phase 12 | ✅ Done | Communication & security hardening (gRPC client / OAuth2 / mTLS / distributed tracing) |
| Phase 13 | ✅ Done | Data backend completion (etcd / Kafka / OpenSearch / InfluxDB) |
| Phase 14 | ✅ Done | Ops & experience (WebSocket / API versioning / Helm / CI/CD) |
| Phase 15 | ✅ Done | Ecosystem expansion v2 (real Kafka / RabbitMQ / MQTT / NATS / MongoDB / S3 / TDengine / OTLP / distributed lock / scheduler / CLI watch+upgrade) |
| Phase 16 | ✅ Done | Maintenance hardening v2.4 (M1 MetricsLayer / M2 RetryLayer / M3 ValidateLayer / M4 CORS / U1 umbrella crate ecat / U2 examples / OAuth2 token hash / CVE tracking) |

## Known Limitations

- **GraphQL parsing (ecat-graphql)**: supports field arguments and nested selections (`query_field`/`mutation_field` rich resolvers can access `args`/`variables`/`selection`); still does not support aliases, fragments, or multiple top-level fields — do not expose it as a general-purpose GraphQL endpoint.
- **OAuth2 introspection cache (ecat-auth)**: the cache key is the SHA-256 hash of the token (the raw token is not stored); cached values are filtered through a whitelist (default keeps sub/exp/iat/role plus iss/aud/scope/roles from extra, configurable via `cache_claims_whitelist`; misses still return full claims, only cached values are filtered); TTL-expired entries are actively purged on write (default TTL 300s).
- **Kafka offsets (ecat-mq-kafka)**: default `enable.auto.commit=false` with no manual commit — after a process restart, messages are re-read from the end of the partition (latest), so messages produced during downtime are skipped; at-least-once semantics (resume from the most recent committed point after restart) require explicitly configuring `auto_commit=true`.

## Design Goals

| # | Goal | Notes |
|---|------|------|
| 1 | **Kratos alignment** | Keep Kratos's API-first, pluggable, unified abstraction philosophy |
| 2 | **Rust idiomatic** | Reuse tower::Service, trait generics, zero-cost abstractions; no "Go in Rust" |
| 3 | **Type safety** | Catch errors at compile time; fully strongly-typed Protobuf definitions |
| 4 | **Pluggable** | Registry, Config, Logging, and Encoding all abstracted through traits |
| 5 | **Complete toolchain** | CLI supports project scaffolding, proto code generation, and dev mode running |
| 6 | **Performance first** | Zero-cost abstractions + async runtime |
| 7 | **Observable** | tracing + Prometheus out of the box |
| 8 | **Complete ecosystem** | Clients, circuit breaker, auth, health checks, registry backends |

## Technical Notes

### Why tower::Service

[`tower::Service`](https://docs.rs/tower/latest/tower/trait.Service.html) is the `http.Handler` equivalent in the Rust async ecosystem. Both axum and tonic are built on tower, so e-cat does not need a custom middleware trait — directly providing tower::Layer implementations achieves the same effect as Kratos middleware with zero adapter overhead.

### Why a Cargo Workspace

Consistent with Kratos's modular design. All `ecat-*` crates are released in lockstep versions within the workspace (currently 3.0.2), each compiled independently, and users import them as needed. Core crates keep dependencies minimal; contrib crates provide optional integrations.

### Why prost (instead of protobuf-rs)

prost is the most widely used protobuf implementation in the Rust community, generating type-safe code at compile time and integrating deeply with tonic.

## Design Documents

- [Design spec](../../../docs/superpowers/specs/2026-07-29-ecat-framework-design.md)
- [Implementation plan](../../../docs/superpowers/plans/2026-07-29-ecat-framework.md)
- [Ecosystem plan v1](ecosystem-plan.md) (done)
- [Ecosystem plan v2](ecosystem-plan-v2.md) (done)
- [Ecosystem plan v3](ecosystem-plan-v3.md) (final evaluation)
- [API reference](api.md)
- [Audit report r5](audit-report-2026-08-01-r5.md) (2026-08-01)
- [Database configuration tutorial](database-config-tutorial.md)
- [Dependency CVE tracking](dependency-cve-tracking.md)
- [TLS certificate authentication tutorial](tls-certificate-tutorial.md)
- [Example config files](../../../config/databases.example.yaml)

## Support

Your support is welcome!

| WeChat Pay | Alipay |
|:---:|:---:|
| <img src="weixinpay.png" width="130" height="130" alt="WeChat Pay"> | <img src="alipay.png" width="130" height="130" alt="Alipay"> |

### Global Transfer (Bank Remittance)

| Item | Details |
|------|------|
| Payee name | WANG KEXUN |
| Payee account number | 881015918251 |
| Payee bank | ZA Bank Limited |
| SWIFT Code | AABLHKHHXXX |
| Bank code | 387 |
| Bank address | Core F, Cyberport 3, 100 Cyberport Road, Hong Kong |

> **Cross-border remittance correspondent bank (if required)**: this is the correspondent (intermediary) bank information, not the payee bank. Please check with your remitting bank whether it needs to be provided.
>
> - For HKD, CNY, and USD remittances: **Citibank N.A. Hong Kong** (SWIFT: `CITIHKHXXXX`, bank code: 006, branch: Hong Kong Branch, branch code: 391, address: Citibank Tower, Citibank Plaza, 3 Garden Road, Central, Hong Kong)
> - For other currencies: **THE BANK OF NEW YORK MELLON** (SWIFT: `IRVTUS3NXXX`, address: 240 GREENWICH STREET, NEW YORK, United States)

## License

Apache-2.0
