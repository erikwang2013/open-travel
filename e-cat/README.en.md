<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# Ecat

[简体中文](README.md) | English | [日本語](docs/i18n/ja/README.md) | [한국어](docs/i18n/ko/README.md) | [Русский](docs/i18n/ru/README.md) | [Deutsch](docs/i18n/de/README.md) | [Français](docs/i18n/fr/README.md) | [Español](docs/i18n/es/README.md) | [Português](docs/i18n/pt/README.md) | [हिन्दी](docs/i18n/hi/README.md) | [العربية](docs/i18n/ar/README.md) | [বাংলা](docs/i18n/bn/README.md) | [Bahasa Indonesia](docs/i18n/id/README.md)

**Ecat** is a Rust microservices framework (v3.0.2 · 51 crates) inspired by [go-kratos/kratos](https://github.com/go-kratos/kratos) v3.

It provides an API-first development experience, pluggable component architecture, unified HTTP/gRPC middleware abstraction, and a complete CLI toolchain. Developers familiar with Kratos can get started immediately, while also leveraging Rust's type safety, zero-cost abstractions, and exceptional performance.

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                         ecat-cli                             │
│        (new │ proto │ run --watch │ build │ upgrade)         │
├──────────────────────────────────────────────────────────────┤
│                     ecat (App Lifecycle)                     │
│      AppBuilder → App { name, servers, hooks, ... }         │
├────────────────────┬────────────────────┬────────────────────┤
│     transport      │    middleware      │     registry       │
│     ─────────      │    ──────────      │     ────────       │
│     HTTP (axum)    │    RecoveryLayer   │     memory         │
│     gRPC (tonic)   │    TracingLayer    │     consul         │
│     WebSocket      │    LoggingLayer    │     etcd           │
│     GraphQL        │    TimeoutLayer    │                    │
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
│                         data layer                            │
│     ────────────────────────────────────────────────          │
│     rdbms:   SQLite / PostgreSQL / MySQL / TiDB              │
│     cache:   Redis / Memcached                               │
│     olap:    ClickHouse                                      │
│     search:  OpenSearch / Elasticsearch                      │
│     graph:   Neo4j / NebulaGraph / ArangoDB                  │
│     tsdb:    InfluxDB / Apache IoTDB / QuestDB / TDengine   │
│     document: MongoDB                                        │
│     storage: S3 / MinIO                                      │
├──────────────────────────────────────────────────────────────┤
│                       ecat-protos                             │
│     (shared .proto definitions: errors, metadata, ...)       │
└──────────────────────────────────────────────────────────────┘
```

### Request Flow

```
Client Request
  │
  ├─ HTTP 0.0.0.0:8000 ──→ axum::Router ──┐
  │                                        │
  └─ gRPC 0.0.0.0:9000 ──→ tonic::Server ─┤
                                           │
                                   ┌───────┴───────┐
                                   │   Middleware   │
                                   │   ──────────   │
                                   │ 1. Recovery    │  catch panics
                                   │ 2. Tracing     │  inject trace_id
                                   │ 3. Logging     │  request logs
                                   │ 4. Auth        │  authn/authz
                                   │ 5. Security    │  attack detection
                                   │ 6. CircuitBrk  │  circuit breaking
                                   └───────┬───────┘
                                           │
                                   ┌───────┴───────┐
                                   │    Handler     │  business logic
                                   │ (tower::Service)│
                                   └───────┬───────┘
                                           │
                                   ┌───────┴───────┐
                                   │   Response     │  encode
                                   │ JSON/Protobuf  │
                                   └───────────────┘
```

## Features

- **API-first**: Protobuf-defined APIs, error codes, and metadata; prost + tonic-build code generation
- **Dual protocol**: HTTP (axum) and gRPC (tonic) sharing one tower::Layer middleware chain
- **Pluggable**: Registry, Config, Logging, Encoding via trait abstractions, production-ready defaults
- **Middleware**: Built-in Recovery, Tracing, Logging, Timeout, RateLimit, Security, CircuitBreaker, MetricsLayer, RetryLayer, ValidateLayer, and CORS (cors feature) layers, composed with tower::ServiceBuilder
- **Lifecycle**: Builder pattern, concurrent servers, SIGTERM/SIGINT handling, start/stop hooks
- **Type-safe**: Protobuf-based error codes with compile-time HTTP status mapping
- **Observable**: tracing + OpenTelemetry (OTLP) + Prometheus + Health endpoints (/health, /ready)
- **Attack detection**: SecurityLayer detects SQL injection, XSS, SSRF patterns and blocks high-risk requests
- **Service comms**: HttpClient/GrpcClient with service discovery and load balancing; CircuitBreaker protection
- **Auth**: JWT / API Key authentication middleware, claims propagated to request context
- **Messaging**: MessageQueue trait + EventBus local/remote Pub/Sub
- **Multi-protocol**: HTTP, gRPC, WebSocket, GraphQL unified routing
- **Multi-source data**: RDBMS (SQLite/PG/MySQL/TiDB), cache (Redis/Memcached), search (OpenSearch/Elasticsearch), graph (Neo4j/NebulaGraph/ArangoDB), TSDB (InfluxDB/IoTDB/QuestDB/TDengine), document (MongoDB), object storage (S3/MinIO)

### Kratos Concept Mapping

| Kratos (Go) | e-cat (Rust) | Notes |
|-------------|-------------|-------|
| `kratos.New()` | `App::builder()` | Builder pattern |
| `http.Handler` | `tower::Service` | Standard Rust ecosystem trait |
| `http.Server` | `axum::Router` | Mainstream HTTP framework |
| `grpc.Server` | `tonic::transport::Server` | Most mature gRPC impl |
| `proto generate` | `prost + tonic-build` | Standard protobuf codegen |
| `registry.Discovery` | `Registry` trait | Pluggable discovery |
| `config.Source` | `ConfigSource` trait | Multi-source config loading |

## Tech Stack

| Component | Choice |
|-----------|--------|
| Runtime | **tokio** |
| HTTP | **axum** |
| gRPC | **tonic** |
| Protobuf | **prost + tonic-build** |
| Middleware | **tower::Service / Layer** |
| Logging/Tracing | **tracing + trace_id propagation** |
| Metrics | **prometheus** |
| Serialization | **serde + prost** |
| Attack detection | **security-rust** |
| RDBMS | **sqlx** |
| Redis | **redis-rs** |
| JWT | **jsonwebtoken** |
| HTTP Client | **reqwest** |
| CLI | **clap** |

## Supported Databases (18 backends)

| Category | Database | Crate | Status |
|----------|----------|-------|--------|
| RDBMS | SQLite | `ecat-data-sqlx` | ✅ Implemented |
| RDBMS | PostgreSQL | `ecat-data-sqlx` | ✅ Implemented |
| RDBMS | MySQL | `ecat-data-sqlx` | ✅ Implemented |
| RDBMS | TiDB | `ecat-data-sqlx` | ✅ Implemented |
| Cache | Redis | `ecat-data-redis` | ✅ Implemented |
| Cache | Memcached | `ecat-data-memcached` | ⚠️ In-memory only (not for production) |
| Search | OpenSearch | `ecat-data-opensearch` | ✅ Implemented |
| Search | Elasticsearch | `ecat-data-elasticsearch` | ✅ Implemented |
| OLAP | ClickHouse | `ecat-data-clickhouse` | ✅ Implemented |
| Graph | Neo4j | `ecat-data-neo4j` | ✅ REST API |
| Graph | NebulaGraph | `ecat-data-nebulagraph` | ✅ REST API |
| Graph | ArangoDB | `ecat-data-arangodb` | ✅ REST API |
| TSDB | InfluxDB | `ecat-data-influxdb` | ✅ HTTP API |
| TSDB | Apache IoTDB | `ecat-data-iotdb` | ✅ REST API |
| TSDB | QuestDB | `ecat-data-questdb` | ✅ HTTP API |
| TSDB | TDengine | `ecat-data-tdengine` | ✅ REST API |
| Document | MongoDB | `ecat-data-mongodb` | ✅ Native driver |
| Object storage | S3 / MinIO | `ecat-data-s3` | ✅ reqwest+rustls |

> All backends share unified trait abstractions (`RdbmsClient` / `Cache` / `SearchClient` / `GraphClient` / `TsdbClient` / `DocumentClient` / `StorageClient`) and provide `XxxConfig` structs (`#[derive(Deserialize)]`) for loading connection info from JSON/YAML config files.

> **Constructor naming convention**: message-queue crates (`ecat-mq-*`) use `connect` as the primary constructor (`KafkaMq::connect(brokers)`, `MqttMq::connect(url)`, …) and additionally offer `from_config`; data-backend crates (`ecat-data-*`) mostly use `new`, except `ecat-data-redis` / `ecat-data-sqlx` (which use `connect`) and `ecat-data-mongodb` / `ecat-data-s3` (which only offer `from_config`). This is an existing convention, not enforced — unification would be breaking and is deferred; it may be revisited in the 3.0 window.

### Messaging Backends (4 MQ)

| Backend | Crate | `from_config` |
|---------|-------|---------------|
| Kafka | `ecat-mq-kafka` | `async` |
| RabbitMQ | `ecat-mq-rabbitmq` | `async` |
| MQTT | `ecat-mq-mqtt` | `async` |
| NATS | `ecat-mq-nats` | `async` |

Also: service registry (Consul / etcd), distributed lock (Redis), scheduler (tokio), OTLP tracing export, API versioning, OpenAPI spec generation.

### Database Configuration

Each backend provides a config struct and `from_config()` method:

```rust
use ecat_data_redis::{RedisCache, RedisConfig};
use ecat_data_sqlx::{SqlxClient, SqlxConfig};

// Load from config file (JSON or YAML)
let redis_cfg: RedisConfig = serde_json::from_str(r#"{"url":"redis://localhost"}"#)?;
let cache = RedisCache::from_config(redis_cfg).await?;

let sql_cfg: SqlxConfig = serde_json::from_str(r#"{"url":"postgres://localhost/db"}"#)?;
let db = SqlxClient::from_config(sql_cfg).await?;
let rows = db.query("SELECT * FROM users").await?;
```

| Backend | Config Struct | Fields |
|---------|--------------|--------|
| Redis | `RedisConfig` | `url`, `password`? |
| RDBMS | `SqlxConfig` | `url`, `username`?, `password`? |
| ClickHouse | `ClickhouseConfig` | `base_url`, `database`, `username`?, `password`? |
| QuestDB | `QuestdbConfig` | `base_url`, `username`?, `password`? |
| Elasticsearch | `ElasticsearchConfig` | `base_url`, `username`?, `password`? |
| OpenSearch | `OpenSearchConfig` | `base_url`, `username`?, `password`? |
| InfluxDB | `InfluxConfig` | `base_url`, `org`, `bucket`, `token` |
| Neo4j | `Neo4jConfig` | `base_url`, `username`, `password` |
| NebulaGraph | `NebulaGraphConfig` | `base_url`, `space`, `username`?, `password`? |
| ArangoDB | `ArangoConfig` | `base_url`, `db`, `username`, `password` |
| IoTDB | `IotdbConfig` | `base_url`, `username`, `password` |
| Memcached | `MemcachedConfig` | `username`?, `password`? (reserved) |
| TDengine | `TdengineConfig` | `base_url`, `username`, `password`, `database`? |
| MongoDB | `MongoConfig` | `url`, `database`, `tls`? |
| S3 | `S3Config` | `endpoint`, `region`, `access_key`, `secret_key`, `tls`? |

> All backends support optional `tls` field (`TlsClientConfig`) for client certificate auth. See [Database Config Tutorial](docs/database-config-tutorial.md) and [TLS Certificate Tutorial](docs/tls-certificate-tutorial.md).

## Project Structure

```
e-cat/
├── ecat/                       # Core: App lifecycle
├── ecat-transport/             # Transport abstraction (Server trait)
├── ecat-transport-http/        # axum implementation
├── ecat-transport-grpc/        # tonic implementation
├── ecat-transport-ws/          # WebSocket transport
├── ecat-middleware/            # tower::Layer middleware
├── ecat-protos/                # Protobuf definitions
├── ecat-errors/                # Error code system
├── ecat-metadata/              # Metadata propagation
├── ecat-encoding/              # Serialization abstraction
├── ecat-logging/               # tracing integration
├── ecat-registry/              # Service registry & discovery
├── ecat-registry-consul/       # Consul registry
├── ecat-registry-etcd/         # etcd registry
├── ecat-config/                # Configuration management
├── ecat-config-remote/         # Consul KV remote config
├── ecat-metrics/               # Prometheus integration
├── ecat-data/                  # Data access traits
├── ecat-data-*/                # 18 database backend crates
├── ecat-security/              # Attack detection (security-rust)
├── ecat-cli/                   # CLI toolchain
├── ecat-health/                # Health checks (/health /ready)
├── ecat-auth/                  # Auth middleware (JWT / API Key / OAuth2)
├── ecat-client/                # HTTP/gRPC service client
├── ecat-circuit-breaker/       # Circuit breaker (Tower Layer)
├── ecat-mq/                    # Message queue abstraction
├── ecat-mq-kafka/              # Kafka adapter
├── ecat-mq-rabbitmq/           # RabbitMQ adapter
├── ecat-mq-mqtt/               # MQTT adapter
├── ecat-mq-nats/               # NATS adapter
├── ecat-events/                # Event bus (local + remote)
├── ecat-testing/               # Integration test tools
├── ecat-openapi/               # OpenAPI spec generation
├── ecat-bench/                 # Performance benchmarks
├── ecat-tracing/               # Distributed tracing (trace_id)
├── ecat-tracing-otlp/          # OpenTelemetry OTLP export
├── ecat-graphql/               # GraphQL endpoint (single-field)
├── ecat-versioning/            # API version routing
├── ecat-tls/                   # TLS config & cert generation
├── ecat-lock/                  # Distributed lock (Redis)
├── ecat-scheduler/             # tokio scheduled tasks
├── ecat-deploy/                # Docker / K8s / Helm / CI/CD
├── config/                     # Example config files
├── docs/                       # Design docs & ecosystem plans
└── examples/                   # Example projects
```

## Quick Start

### Prerequisites

- Rust 1.85+ (stable toolchain, required for edition 2024)
- [protoc](https://github.com/protocolbuffers/protobuf) (Protocol Buffers compiler)

### Install the CLI

```bash
cargo install ecat-cli
```

### Create a Service

```bash
# Scaffold a project
ecat new helloworld
cd helloworld

# Add a proto definition
ecat proto add proto/service.proto

# Generate client and server code
ecat proto client proto/service.proto
ecat proto server proto/service.proto -t internal/service

# Run in development mode
ecat run

# Watch src/ for changes and auto-restart
ecat run --watch

# Build for production
ecat build --release

# Update all ecat-* workspace dependencies
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

    app.run().await?; // blocks until SIGTERM/SIGINT
    Ok(())
}
```

> Note: use `0.0.0.0:port` (not `:port`) as the listen address so the service
> also binds correctly on hosts without IPv6.

### Aggregation crate (ecat)

`ecat` provides feature-gated re-export entry points — enable only the components you need:

```rust
use ecat::transport_http::HttpServer;   // feature "http" (default)
use ecat::middleware::RecoveryLayer;     // feature "middleware"
use ecat::auth::JwtAuthLayer;            // feature "auth"
use ecat::data::redis::RedisCache;       // feature "redis"
```

Default features = `http+grpc`; use `--no-default-features --features <component>` to slim the dependency tree. Full feature list: `http` `grpc` `middleware` `auth` `client` `events` `metrics` `tracing` `circuit-breaker` `consul` `remote` `redis`.

### Middleware

```rust
use tower::ServiceBuilder;
use ecat_middleware::{RecoveryLayer, TracingLayer, LoggingLayer, TimeoutLayer};
use ecat_circuit_breaker::CircuitBreakerLayer;
use ecat_security::SecurityLayer;
use ecat_auth::JwtAuthLayer;
use std::time::Duration;

// JWT secret must be ≥32 bytes; optionally enforce iss/aud claims (not enforced by default):
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

> Note: `ecat_middleware::TracingLayer` does not inject `trace_id`; use `ecat_tracing::TracingLayer::new()` for request-level `trace_id` injection.

```rust
// Metrics: records request count and duration into the global registry
// (shared with the /metrics endpoint)
use ecat_metrics::MetricsLayer;
let app = Router::new().route("/hello", get(hello)).layer(MetricsLayer::new());
// Metric names: ecat_http_requests_total / ecat_http_request_duration_seconds
// (labels: method/path/status). For high-cardinality paths (with IDs), use
// MetricsLayer::new().with_path_fn(...) to normalize, avoiding cardinality explosion.

// Retry: exponential backoff; ⚠️ only safe for idempotent requests (GET/HEAD/PUT/DELETE)
use ecat_middleware::RetryLayer;
let retry = RetryLayer::new(3, Duration::from_secs(1), Duration::from_secs(30)); // 3 total attempts incl. first
// Custom retry rule: RetryLayer::new(3, ...).with_rule(MyRule)  // decide by status code / response content

// Validation: check headers/params before routing; short-circuits with a JSON error (400 by default, with_status can set 422 etc.)
use ecat_middleware::{ValidateLayer, ValidateError};
let validate = ValidateLayer::from_fn(|req: &http::Request<axum::body::Body>| {
    if req.headers().contains_key("x-api-key") {
        Ok(())
    } else {
        Err(ValidateError::new("missing x-api-key").with_status(422))
    }
});

// CORS: requires the "cors" feature on ecat-middleware
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

## Implementation Progress

| Phase | Status | Content |
|-------|--------|---------|
| Phase 1 | ✅ Done | Project skeleton, protos, errors, metadata, encoding, logging |
| Phase 2 | ✅ Done | Transport layer (HTTP + gRPC) |
| Phase 3 | ✅ Done | Middleware (Recovery/Tracing/Logging/Timeout) |
| Phase 4 | ✅ Done | App lifecycle management |
| Phase 5 | ✅ Done | Registry, Config, Metrics |
| Phase 5.5 | ✅ Done | Data access layer (traits + sqlx backend) |
| Phase 6 | ✅ Done | CLI toolchain (new/proto/run/build) |
| Phase 7 | ✅ Done | README, examples (helloworld), design docs |
| Phase 8 | ✅ Done | Attack detection (security-rust, ecat-security) |
| Phase 9 | ✅ Done | Ecosystem I (health / client / circuit-breaker / auth / consul) |
| Phase 10 | ✅ Done | Ecosystem II (redis / mq / events / config-remote) |
| Phase 11 | ✅ Done | Ecosystem III (testing / deploy / bench / openapi) |
| Phase 12 | ✅ Done | Comms & security (gRPC client / OAuth2 / mTLS / tracing) |
| Phase 13 | ✅ Done | Data backends (etcd / Kafka / OpenSearch / InfluxDB / ES / ClickHouse / Memcached / Neo4j / NebulaGraph / ArangoDB / IoTDB / QuestDB) |
| Phase 14 | ✅ Done | Ops & UX (WebSocket / API versioning / GraphQL / Helm / CI/CD) |
| Phase 15 | ✅ Done | Ecosystem v2 (real Kafka / RabbitMQ / MQTT / NATS / MongoDB / S3 / TDengine / OTLP / distributed lock / scheduler / CLI watch+upgrade) |
| Phase 16 | ✅ Done | Maintenance v2.4 (M1 MetricsLayer / M2 RetryLayer / M3 ValidateLayer / M4 CORS / U1 aggregation crate ecat / U2 examples / OAuth2 token hash / CVE tracking) |

## Known Limitations

- **GraphQL resolution (ecat-graphql)**: field arguments and nested selections are supported (`query_field`/`mutation_field` rich resolvers receive `args`/`variables`/`selection`); aliases, fragments and multiple top-level fields are still rejected — do not expose it as a general-purpose GraphQL endpoint.
- **OAuth2 introspection cache (ecat-auth)**: the cache key is a SHA-256 hash of the token (no plaintext token stored); cached values are whitelist-filtered (default keeps sub/exp/iat/role plus extra iss/aud/scope/roles, configurable via `cache_claims_whitelist`; misses still return full claims, only cached values are filtered); expired entries are actively purged on write (default TTL 300s).
- **Kafka offset handling (ecat-mq-kafka)**: `enable.auto.commit=false` by default with no manual commit — after a restart the consumer re-reads from the partition end (latest), skipping messages produced while down; explicitly set `auto_commit=true` for at-least-once semantics (resumes from the last committed point).

## Design Goals

| # | Goal | Notes |
|---|------|-------|
| 1 | **Kratos alignment** | API-first, pluggable, unified abstractions |
| 2 | **Rust idiomatic** | tower::Service, trait generics, zero-cost abstractions; no "Go in Rust" |
| 3 | **Type safety** | Compile-time errors, fully typed Protobuf definitions |
| 4 | **Pluggable** | Registry, Config, Logging, Encoding via traits |
| 5 | **Complete toolchain** | CLI scaffolding, proto codegen, dev run, upgrade |
| 6 | **Performance first** | Zero-cost abstractions + async runtime |
| 7 | **Observable** | tracing + Prometheus out of the box |
| 8 | **Complete ecosystem** | Clients, circuit breaker, auth, health checks, registry backends |

## Technical Notes

### Why tower::Service

[`tower::Service`](https://docs.rs/tower/latest/tower/trait.Service.html) is the Rust async ecosystem equivalent of `http.Handler`. Both axum and tonic are built on tower, so e-cat needs no custom middleware trait — implement tower::Layer directly, with zero adapter overhead.

### Why a Cargo Workspace

Consistent with Kratos' modular design. All `ecat-*` crates release with lockstep workspace versioning (currently 3.0.2), compiling independently; users pull in only what they need. Core crates keep minimal dependencies; contrib crates provide optional integrations.

### Why prost (not protobuf-rs)

prost is the most widely used protobuf implementation in the Rust community, generating type-safe code at compile time with deep tonic integration.

## Documentation

- [Design Spec](docs/superpowers/specs/2026-07-29-ecat-framework-design.md)
- [Implementation Plan](docs/superpowers/plans/2026-07-29-ecat-framework.md)
- [Ecosystem Plan v1](docs/ecosystem-plan.md) (completed)
- [Ecosystem Plan v2](docs/ecosystem-plan-v2.md) (completed)
- [Ecosystem Plan v3](docs/ecosystem-plan-v3.md) (final evaluation)
- [API Reference](docs/api.md)
- [Audit Report r5](docs/audit-report-2026-08-01-r5.md) (2026-08-01)
- [Database Config Tutorial](docs/database-config-tutorial.md)
- [Dependency CVE Tracking](docs/dependency-cve-tracking.md)
- [TLS Certificate Tutorial](docs/tls-certificate-tutorial.md)
- [Config Example](config/databases.example.yaml)

## Support

Your support is welcome!

| WeChat Pay | Alipay |
|:---:|:---:|
| <img src="docs/weixinpay.png" width="130" height="130" alt="WeChat Pay"> | <img src="docs/alipay.png" width="130" height="130" alt="Alipay"> |

### Global Transfer (Bank Wire)

| Field | Value |
|-------|-------|
| Beneficiary Name | WANG KEXUN |
| Account Number | 881015918251 |
| Bank | ZA Bank Limited |
| SWIFT Code | AABLHKHHXXX |
| Bank Code | 387 |
| Bank Address | Core F, Cyberport 3, 100 Cyberport Road, Hong Kong |

> **Cross-border remittance agent bank (if required)**: this is the agent (intermediary) bank information, NOT the receiving bank. Ask your remitting bank whether it is required.
>
> - For HKD, CNY and USD remittances: **Citibank N.A. Hong Kong** (SWIFT: `CITIHKHXXXX`, Bank Code: 006, Branch: Hong Kong Branch, Branch Code: 391, Address: Citibank Tower, Citibank Plaza, 3 Garden Road, Central, Hong Kong)
> - For other currencies: **THE BANK OF NEW YORK MELLON** (SWIFT: `IRVTUS3NXXX`, Address: 240 GREENWICH STREET, NEW YORK, United States)

## License

Apache-2.0
