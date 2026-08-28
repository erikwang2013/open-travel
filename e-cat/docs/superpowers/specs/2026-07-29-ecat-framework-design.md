<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# e-cat Framework Design Spec

**Date:** 2026-07-29
**Status:** Draft
**Author:** Erik

## 1. 概述

e-cat 是对标 [go-kratos/kratos](https://github.com/go-kratos/kratos) v3 的 Rust 微服务框架。提供 API-first 开发体验、可插拔的组件架构、统一的 HTTP/gRPC 中间件抽象，以及完备的 CLI 工具链。

### 1.1 设计目标

| # | 目标 | 说明 |
|---|------|------|
| 1 | **Kratos 对齐** | 保持 Kratos 的 API-first、可插拔、统一抽象理念 |
| 2 | **Rust 惯用** | 复用 tower::Service、trait 泛型、零成本抽象；不做 "Go in Rust" |
| 3 | **类型安全** | 利用 Rust 类型系统，编译期捕获错误；Protobuf 定义全强类型化 |
| 4 | **可插拔** | Registry、Config、Logging、Encoding 全部通过 trait 抽象 |
| 5 | **工具链完备** | CLI 支持项目脚手架、proto 代码生成、开发运行 |
| 6 | **性能优先** | 零成本抽象 + 异步运行时，提供 Go 无法企及的吞吐和延迟 |
| 7 | **可观测** | tracing + OpenTelemetry + Prometheus 开箱即用 |

### 1.2 技术栈

| 组件 | 选型 | 对标 Kratos |
|------|------|------------|
| 异步运行时 | **tokio** | goroutine |
| HTTP 框架 | **axum** | net/http (blademaster) |
| gRPC 框架 | **tonic** | gRPC-Go (warden) |
| Protobuf 代码生成 | **prost + tonic-build** | protoc-gen-go |
| 中间件抽象 | **tower::Service / Layer** | kratos.Middleware |
| 日志/追踪 | **tracing + opentelemetry-rust** | log/slog + OTel |
| 指标 | **prometheus** | Prometheus client |
| 序列化 | **serde + prost** | encoding/json + proto |
| CLI | **clap** | cobra |
| 缓存 | **redis-rs / memcache-rs** | redis/go-redis |
| 搜索 | **opensearch-rs / elasticsearch-rs** | elastic/go-elasticsearch |
| OLAP | **clickhouse-rs** | clickhouse-go |
| 图数据库 | **neo4rs / nebula-client / arangors** | neo4j-go-driver |
| 时序数据库 | **influxdb2 / iotdb-client-rs / questdb-rs** | influxdb-client-go |
| RDBMS | **sqlx** (SQLite/PG/MySQL/TiDB) | database/sql + drivers |

## 2. 架构设计

### 2.1 分层架构

```
┌──────────────────────────────────────────────────┐
│                  ecat-cli                        │  ← CLI 工具链
│        (new | proto | run | build)               │
├──────────────────────────────────────────────────┤
│              ecat (App 生命周期)                   │  ← 应用编排层
│    AppBuilder → App { http_srv, grpc_srv, ... }  │
├──────────────┬──────────────┬────────────────────┤
│  transport   │  middleware  │     registry       │  ← 核心组件层
│  ─────────   │  ─────────   │     ────────       │
│  HTTP/gRPC   │  recovery    │     etcd/consul    │
│  encoding    │  tracing     │     dns/memory     │
│              │  auth/...    │                    │
├──────────────┼──────────────┼────────────────────┤
│   config     │   errors     │     metadata       │  ← 基础设施层
├──────────────┴──────────────┴────────────────────┤
│                    data                          │  ← 数据访问层
│  ─────────────────────────────────────           │
│  cache:    Redis / Memcached                     │
│  olap:     ClickHouse                            │
│  search:   OpenSearch / Elasticsearch             │
│  graph:    Neo4j / NebulaGraph / ArangoDB        │
│  tsdb:     InfluxDB / IoTDB / QuestDB            │
├──────────────────────────────────────────────────┤
│              ecat-protos                         │  ← IDL 定义
│    (共享 protobuf 定义: errors, metadata, ...)    │
└──────────────────────────────────────────────────┘
```

### 2.2 Cargo Workspace 结构

```
e-cat/                          # workspace root
├── Cargo.toml                  # [workspace] members
├── ecat/                       # 核心 crate：应用生命周期
├── ecat-transport/             # HTTP + gRPC 传输抽象
├── ecat-transport-http/        # axum 实现
├── ecat-transport-grpc/        # tonic 实现
├── ecat-middleware/            # 通用中间件（tower::Layer）
├── ecat-registry/              # 服务注册/发现接口
├── ecat-registry-etcd/         # etcd 注册实现（contrib）
├── ecat-config/                # 配置管理
├── ecat-config-file/           # 文件配置源
├── ecat-logging/               # tracing 集成
├── ecat-encoding/              # 编码（JSON/Protobuf）
├── ecat-errors/                # 错误码定义
├── ecat-metadata/              # 元数据传递
├── ecat-metrics/               # Prometheus 集成
├── ecat-data/                  # 数据访问 trait 抽象
├── ecat-data-sqlx/             # RDBMS 统一客户端（sqlx：SQLite/PG/MySQL/TiDB）
├── ecat-data-redis/            # Redis 客户端（contrib）
├── ecat-data-memcached/        # Memcached 客户端（contrib）
├── ecat-data-clickhouse/       # ClickHouse 客户端（contrib）
├── ecat-data-opensearch/       # OpenSearch 客户端（contrib）
├── ecat-data-elasticsearch/    # Elasticsearch 客户端（contrib）
├── ecat-data-neo4j/            # Neo4j 客户端（contrib）
├── ecat-data-nebulagraph/      # NebulaGraph 客户端（contrib）
├── ecat-data-arangodb/         # ArangoDB 客户端（contrib）
├── ecat-data-influxdb/         # InfluxDB 客户端（contrib）
├── ecat-data-iotdb/            # Apache IoTDB 客户端（contrib）
├── ecat-data-questdb/          # QuestDB 客户端（contrib）
├── ecat-protos/                # Protobuf 定义
├── ecat-security/              # 攻击检测（security-rust 27 检测器）
├── ecat-cli/                   # CLI 工具
└── examples/                   # 示例项目
```

### 2.3 请求处理流程

```
客户端
  │
  ├─ HTTP/1.1 :8000 ──→ axum::Router
  │                        │
  └─ gRPC  :9000  ──→ tonic::Server
                           │
                    ┌──────┴──────┐
                    │  Middleware  │  ← tower::Layer 链
                    │  ─────────   │
                    │  1. Recovery │     捕获 panic
                    │  2. Tracing  │     注入 trace_id
                    │  3. Logging  │     请求日志
                    │  4. Auth     │     认证/鉴权
                    │  5. Metrics  │     指标采集
                    │  6. Security │     攻击检测
                    └──────┬──────┘
                           │
                    ┌──────┴──────┐
                    │   Handler   │  ← 用户业务逻辑 (tower::Service)
                    └──────┬──────┘
                           │
                    ┌──────┴──────┐
                    │   Response  │  ← 编码/序列化 (JSON/Protobuf)
                    └─────────────┘
```

### 2.4 Kratos 概念 → Rust 实现映射

| Kratos (Go) | e-cat (Rust) | 设计理由 |
|-------------|-------------|----------|
| `kratos.New()` | `ecat::App::builder()` | Builder 模式更 Rust 化 |
| `http.Handler` | `tower::Service` | 复用 Rust 生态标准 |
| `http.Server` | `axum::Router` | 社区最主流 HTTP 框架 |
| `grpc.Server` | `tonic::transport::Server` | 最成熟 gRPC 实现 |
| `proto generate` | `prost + tonic-build` | Rust 社区标准 protobuf |
| `registry.Discovery` | `Registry trait` | 可插拔注册发现 |
| `config.Source` | `ConfigSource trait` | 多源配置加载 |

## 3. 核心接口设计

### 3.1 App 生命周期

```rust
pub struct App {
    name: String,
    version: String,
    servers: Vec<Arc<dyn Server>>,
    start_hooks: Vec<Box<dyn LifecycleHook>>,
    stop_hooks: Vec<Box<dyn LifecycleHook>>,
}

impl App {
    pub fn builder() -> AppBuilder;
    pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}
```

### 3.2 Registry（服务注册发现）

```rust
#[async_trait]
pub trait Registry: Send + Sync {
    async fn register(&self, service: &ServiceInfo) -> Result<Registration>;
    async fn discover(&self, name: &str) -> Result<Vec<ServiceInfo>>;
    async fn watch(&self, name: &str) -> Result<WatchStream>;
}
```

### 3.3 ConfigSource（配置源）

```rust
#[async_trait]
pub trait ConfigSource: Send + Sync {
    async fn load(&self) -> Result<HashMap<String, Value>>;
    async fn watch(&self) -> Result<WatchStream>;
}
```

### 3.4 LifecycleHook（生命周期钩子）

```rust
#[async_trait]
pub trait LifecycleHook: Send + Sync {
    async fn on_start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn on_stop(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}
```

### 3.5 Middleware（tower::Layer）

```rust
// e-cat 提供标准 tower::Layer 实现，不做自定义 middleware trait
// 内置 layer：RecoveryLayer、TracingLayer、LoggingLayer、MetricsLayer、TimeoutLayer、SecurityLayer

let layer = tower::ServiceBuilder::new()
    .layer(RecoveryLayer)
    .layer(TracingLayer)
    .layer(LoggingLayer)
    .layer(TimeoutLayer::new(Duration::from_secs(30)))
    .layer(SecurityLayer::new());
```

### 3.6 典型使用示例

```rust
use ecat::App;

#[tokio::main]
async fn main() -> Result<()> {
    let http_srv = HttpServer::new(":8000");
    let grpc_srv = GrpcServer::new(":9000");

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

    app.run().await?;  // blocks until SIGTERM
    Ok(())
}
```

### 3.7 Data（数据访问抽象）

数据存储按类别统一 trait，每个后端提供 contrib crate 实现。

```rust
// RDBMS (SQLite / PostgreSQL / MySQL / TiDB)
// 统一使用 sqlx —— 编译期 SQL 校验、异步、多数据库支持
#[async_trait]
pub trait RdbmsClient: Send + Sync {
    async fn execute(&self, sql: &str, args: &[Value]) -> Result<u64>;
    async fn query(&self, sql: &str, args: &[Value]) -> Result<Vec<Row>>;
    async fn transaction(&self) -> Result<Transaction>;
}

// 缓存 (Redis / Memcached)
#[async_trait]
pub trait Cache: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>>;
    async fn set(&self, key: &str, value: &[u8], ttl: Duration) -> Result<()>;
    async fn delete(&self, key: &str) -> Result<()>;
    async fn exists(&self, key: &str) -> Result<bool>;
}

// OLAP 分析 (ClickHouse)
#[async_trait]
pub trait OlapClient: Send + Sync {
    async fn query(&self, sql: &str) -> Result<QueryResult>;
    async fn insert(&self, table: &str, rows: &[Row]) -> Result<()>;
}

// 搜索引擎 (OpenSearch / Elasticsearch)
#[async_trait]
pub trait SearchClient: Send + Sync {
    async fn index(&self, index: &str, id: &str, doc: &Value) -> Result<()>;
    async fn search(&self, index: &str, query: &Value) -> Result<SearchResult>;
    async fn delete(&self, index: &str, id: &str) -> Result<()>;
}

// 图数据库 (Neo4j / NebulaGraph / ArangoDB)
#[async_trait]
pub trait GraphClient: Send + Sync {
    async fn execute(&self, query: &str, params: &Value) -> Result<GraphResult>;
    async fn execute_batch(&self, queries: &[&str]) -> Result<Vec<GraphResult>>;
}

// 时序数据库 (InfluxDB / IoTDB / QuestDB)
#[async_trait]
pub trait TsdbClient: Send + Sync {
    async fn write(&self, measurement: &str, points: &[DataPoint]) -> Result<()>;
    async fn query(&self, query: &str) -> Result<QueryResult>;
}
```

**各类别 Rust 驱动选型：**

| 类别 | 存储 | Rust 驱动 |
|------|------|----------|
| RDBMS | SQLite / PostgreSQL / MySQL / TiDB | **sqlx** |
| Cache | Redis | **redis-rs** |
| Cache | Memcached | **memcache-rs** |
| OLAP | ClickHouse | **clickhouse-rs** |
| Search | OpenSearch | **opensearch-rs** |
| Search | Elasticsearch | **elasticsearch-rs** |
| Graph | Neo4j | **neo4rs** |
| Graph | NebulaGraph | **nebula-client** |
| Graph | ArangoDB | **arangors** |
| TSDB | InfluxDB | **influxdb2** |
| TSDB | Apache IoTDB | **iotdb-client-rs** |
| TSDB | QuestDB | **questdb-rs** (ILP protocol) |

## 4. 实现步骤

### Phase 1：项目骨架

- 创建 workspace Cargo.toml，配置 members
- 搭建 `ecat-protos` crate —— errors.proto、metadata.proto
- 搭建 `ecat-errors` crate —— 基于 protobuf 的错误码体系
- 搭建 `ecat-metadata` crate —— 统一的元数据 key-value 传递
- 搭建 `ecat-encoding` crate —— JSON + Protobuf 序列化
- 搭建 `ecat-logging` crate —— tracing 集成 + 日志接口
- CI 配置（GitHub Actions）：build + test + clippy + fmt

### Phase 2：Transport 层

- 搭建 `ecat-transport` crate —— Transport trait 和 Server trait
- 搭建 `ecat-transport-http` crate —— axum 集成
- 搭建 `ecat-transport-grpc` crate —— tonic 集成
- HTTP/gRPC 双向元数据传递（header <-> metadata）
- 优雅停机（graceful shutdown）

### Phase 3：Middleware 体系

- 搭建 `ecat-middleware` crate —— tower::Layer 中间件
- recovery 中间件（catch panic）
- tracing 中间件（trace_id / span_id）
- logging 中间件（请求日志）
- metrics 中间件（Prometheus 采集）
- timeout 中间件
- tower::ServiceBuilder 组合器

### Phase 4：App 生命周期

- 搭建 `ecat` 核心 crate
- AppBuilder → App 生命周期管理
- 多 Server 并发启动
- OS 信号处理（SIGTERM/SIGINT）
- LifecycleHook 机制

### Phase 5：Registry & Config

- 搭建 `ecat-registry` crate —— Registry trait + 内存实现
- 搭建 `ecat-registry-etcd` contrib crate
- 搭建 `ecat-config` crate —— ConfigSource trait
- file/env 配置源
- 配置热加载（watch 机制）
- 搭建 `ecat-metrics` crate —— Prometheus 集成

### Phase 5.5：Data 数据访问层

- 搭建 `ecat-data` crate —— RdbmsClient / Cache / OlapClient / SearchClient / GraphClient / TsdbClient trait
- 搭建 `ecat-data-sqlx` crate（sqlx：SQLite / PostgreSQL / MySQL / TiDB）
- 搭建 `ecat-data-redis` crate（redis-rs）
- 搭建 `ecat-data-memcached` crate（memcache-rs）
- 搭建 `ecat-data-clickhouse` crate（clickhouse-rs）
- 搭建 `ecat-data-opensearch` crate（opensearch-rs）
- 搭建 `ecat-data-elasticsearch` crate（elasticsearch-rs）
- 搭建 `ecat-data-neo4j` crate（neo4rs）
- 搭建 `ecat-data-nebulagraph` crate（nebula-client）
- 搭建 `ecat-data-arangodb` crate（arangors）
- 搭建 `ecat-data-influxdb` crate（influxdb2）
- 搭建 `ecat-data-iotdb` crate（iotdb-client-rs）
- 搭建 `ecat-data-questdb` crate（questdb-rs / ILP）

### Phase 6：CLI 工具

- 搭建 `ecat-cli` crate（clap）
- `ecat new <name>` —— 项目脚手架
- `ecat proto add|client|server <file>` —— proto 代码生成
- `ecat run` —— 开发模式
- `ecat build` —— 生产构建

### Phase 7：生态 & 文档

- mdBook 参考文档
- examples/helloworld 示例
- ecat-layout 项目模板仓库
- contrib crate：registry-consul、config-nacos、middleware-auth、以及 Phase 5.5 中全部 11 个 data-* crate
- benchmarks/ 性能基准
- 发布 crates.io
