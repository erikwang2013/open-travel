<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# Ecat

[简体中文](../../../README.md) | [English](../../../README.en.md) | [日本語](../ja/README.md) | [한국어](../ko/README.md) | [Русский](../ru/README.md) | [Deutsch](../de/README.md) | [Français](../fr/README.md) | [Español](../es/README.md) | [Português](../pt/README.md) | [हिन्दी](../hi/README.md) | [العربية](../ar/README.md) | [বাংলা](../bn/README.md) | **Bahasa Indonesia** | 简体中文

Nama Tionghoa Ecat: 一只猫 (seekor kucing)

**Ecat** adalah framework microservice Rust yang sejajar dengan [go-kratos/kratos](https://github.com/go-kratos/kratos) v3 (v3.0.2 · 51 crates).

Menawarkan pengalaman pengembangan API-first, arsitektur komponen yang dapat dipasang, abstraksi middleware HTTP/gRPC terpadu, serta rantai alat CLI yang lengkap. Pengembang yang akrab dengan Kratos dapat langsung menggunakannya, sekaligus memanfaatkan sepenuhnya type-safety Rust, abstraksi biaya nol, dan performa ekstrem.

<p align="center">
  <img src="e-cat.svg" alt="Maskot proyek Ecat (dinamis)" width="220" />
</p>

## Arsitektur Desain

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

### Alur Pemrosesan Permintaan

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

## Fitur

- **API-first**: Mendefinisikan API, kode error, dan metadata dengan Protobuf; pembuatan kode melalui prost + tonic-build
- **Dukungan protokol ganda**: HTTP (axum) dan gRPC (tonic) berbagi set middleware `tower::Layer` yang sama
- **Arsitektur pluggable**: Registry, Config, Logging, Encoding semuanya diabstraksikan melalui trait, dengan implementasi siap-produksi disediakan secara default
- **Sistem middleware**: Built-in Recovery, Tracing, Logging, Timeout, RateLimit, Security, CircuitBreaker, MetricsLayer, RetryLayer, ValidateLayer, CORS (fitur `cors`); dikombinasikan melalui `tower::ServiceBuilder`
- **Siklus hidup aplikasi**: Membangun App dengan pola Builder, memulai banyak Server secara bersamaan, penanganan sinyal SIGTERM/SIGINT, hook siklus hidup start/stop
- **Type-safety**: Sistem kode error berbasis protobuf, pemetaan status HTTP pada waktu kompilasi
- **Observabilitas**: tracing + Prometheus + endpoint Health (/health, /ready)
- **Deteksi serangan**: SecurityLayer secara otomatis mendeteksi pola serangan seperti SQL injection, XSS, SSRF, dan memblokir permintaan berisiko tinggi
- **Komunikasi antar-layanan**: HttpClient terintegrasi dengan service discovery dan load balancing, perlindungan circuit breaker dengan CircuitBreaker
- **Autentikasi & otorisasi**: Middleware autentikasi JWT / API Key, Claims diteruskan ke konteks permintaan
- **Pesan & peristiwa**: trait MessageQueue + EventBus Pub/Sub lokal/jarak jauh
- **Pelacakan terdistribusi**: span permintaan, injeksi/ekstraksi trace_id
- **Klien gRPC**: GrpcClient terintegrasi dengan service discovery dan load balancing
- **Multi-protokol**: HTTP, gRPC, WebSocket, GraphQL dengan routing terpadu
- **Multi-sumber data**: RDBMS (SQLite/PG/MySQL/TiDB), cache (Redis/Memcached), pencarian (OpenSearch/Elasticsearch), graf (Neo4j/NebulaGraph/ArangoDB), time-series (InfluxDB/IoTDB/QuestDB/TDengine), dokumen (MongoDB), object storage (S3/MinIO)

### Pemetaan Konsep Kratos

| Kratos (Go) | e-cat (Rust) | Keterangan |
|-------------|-------------|------|
| `kratos.New()` | `App::builder()` | Pola Builder |
| `http.Handler` | `tower::Service` | Trait standar ekosistem Rust |
| `http.Server` | `axum::Router` | Framework HTTP mainstream komunitas |
| `grpc.Server` | `tonic::transport::Server` | Implementasi gRPC paling matang |
| `proto generate` | `prost + tonic-build` | Protobuf standar komunitas |
| `registry.Discovery` | `Registry` trait | Registri & discovery pluggable |
| `config.Source` | `ConfigSource` trait | Pemuatan konfigurasi multi-sumber |

## Tumpukan Teknologi

| Komponen | Pilihan |
|------|------|
| Runtime asinkron | **tokio** |
| HTTP | **axum** |
| gRPC | **tonic** |
| Protobuf | **prost + tonic-build** |
| Middleware | **tower::Service / Layer** |
| Logging/tracing | **tracing + trace_id propagation** |
| Metrik | **prometheus** |
| Serialisasi | **serde + prost** |
| Deteksi serangan | **security-rust** |
| RDBMS | **sqlx** |
| Redis | **redis-rs** |
| JWT | **jsonwebtoken** |
| HTTP Client | **reqwest** |
| CLI | **clap** |

## Database yang Didukung

| Kategori | Database | Crate | Status |
|------|--------|-------|------|
| RDBMS | SQLite | `ecat-data-sqlx` | ✅ Terimplementasi |
| RDBMS | PostgreSQL | `ecat-data-sqlx` | ✅ Terimplementasi |
| RDBMS | MySQL | `ecat-data-sqlx` | ✅ Terimplementasi |
| RDBMS | TiDB | `ecat-data-sqlx` | ✅ Terimplementasi |
| Cache | Redis | `ecat-data-redis` | ✅ Terimplementasi |
| Pencarian | OpenSearch | `ecat-data-opensearch` | ✅ Terimplementasi |
| Pencarian | Elasticsearch | `ecat-data-elasticsearch` | ✅ Terimplementasi |
| Cache | Memcached | `ecat-data-memcached` | ⚠️ Implementasi memori (bukan produksi, jangan gunakan untuk cache persisten) |
| OLAP | ClickHouse | `ecat-data-clickhouse` | ✅ Terimplementasi |
| Graf | Neo4j | `ecat-data-neo4j` | ✅ REST API |
| Graf | NebulaGraph | `ecat-data-nebulagraph` | ✅ REST API |
| Graf | ArangoDB | `ecat-data-arangodb` | ✅ REST API |
| Time-series | InfluxDB | `ecat-data-influxdb` | ✅ HTTP API |
| Time-series | Apache IoTDB | `ecat-data-iotdb` | ✅ REST API |
| Time-series | QuestDB | `ecat-data-questdb` | ✅ HTTP API |
| Time-series | TDengine | `ecat-data-tdengine` | ✅ REST API |
| Dokumen | MongoDB | `ecat-data-mongodb` | ✅ Driver native |
| Object storage | S3 / MinIO | `ecat-data-s3` | ✅ reqwest+rustls |

> Semua backend data diabstraksikan melalui trait terpadu (`RdbmsClient` / `Cache` / `SearchClient` / `GraphClient` / `TsdbClient` / `DocumentClient` / `StorageClient`), impor crate contrib terkait sesuai kebutuhan. Setiap backend menyediakan struct `XxxConfig` (`#[derive(Deserialize)]`), yang mendukung pemuatan info koneksi dari file konfigurasi JSON/YAML.

> **Konvensi penamaan konstruktor**: konstruktor utama crate message queue (`ecat-mq-*`) terpadu sebagai `connect` (mis. `KafkaMq::connect(brokers)`, `MqttMq::connect(url)`), juga menyediakan `from_config` untuk memuat dari konfigurasi; sebagian besar konstruktor utama crate backend data (`ecat-data-*`) adalah `new`, dengan pengecualian: `ecat-data-redis` / `ecat-data-sqlx` tetap menggunakan `connect`, `ecat-data-mongodb` / `ecat-data-s3` hanya menyediakan `from_config`. Ini adalah konvensi yang sudah ada, tidak dipaksakan untuk diseragamkan (menghindari perubahan yang merusak); dapat dievaluasi untuk diseragamkan di jendela 3.0.

### Contoh Konfigurasi Database

Setiap backend data menyediakan struct `XxxConfig` dan metode `from_config()`, untuk memisahkan info koneksi dari kode ke file konfigurasi:

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

**Referensi kolom konfigurasi**:

| Backend | Config | Kolom | Contoh nilai |
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
| Memcached | `MemcachedConfig` | `username`?, `password`? (kolom cadangan) | — |
| TDengine | `TdengineConfig` | `base_url`, `username`, `password`, `database`? | `http://localhost:6041` |
| MongoDB | `MongoConfig` | `url`, `database`, `tls`? | `mongodb://localhost:27017`, `app` |
| S3 | `S3Config` | `endpoint`, `region`, `access_key`, `secret_key`, `tls`? | `http://localhost:9000`, `us-east-1` |

> Semua Config backend mendukung kolom opsional `tls` (`TlsClientConfig`) untuk mengonfigurasi autentikasi sertifikat klien TLS. Lihat [Tutorial Konfigurasi Database](database-config-tutorial.md).

## Struktur Proyek

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

## Memulai Cepat

### Prasyarat

- Rust 1.85+ (toolchain stable, syarat edition 2024)
- [protoc](https://github.com/protocolbuffers/protobuf) (kompiler Protocol Buffers)

### Instalasi CLI

```bash
cargo install ecat-cli
```

### Membuat Layanan

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

Akses `http://localhost:8000/helloworld/ecat`.

### Contoh Kode

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

### Crate Agregat (ecat)

`ecat` menyediakan titik re-export dengan feature-gated — hanya aktifkan komponen yang dibutuhkan:

```rust
use ecat::transport_http::HttpServer;   // feature "http"（默认）
use ecat::middleware::RecoveryLayer;     // feature "middleware"
use ecat::auth::JwtAuthLayer;            // feature "auth"
use ecat::data::redis::RedisCache;       // feature "redis"
```

Default features = `http+grpc`; gunakan `--no-default-features --features <komponen>` untuk mengecilkan pohon dependensi. Daftar feature lengkap: `http` `grpc` `middleware` `auth` `client` `events` `metrics` `tracing` `circuit-breaker` `consul` `remote` `redis`.

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

> Catatan: `ecat_middleware::TracingLayer` tidak menyuntikkan trace_id; jika membutuhkan injeksi trace_id tingkat permintaan, gunakan `ecat_tracing::TracingLayer::new()`.

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

### Penanganan Error

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

## Tahap Implementasi

| Tahap | Status | Isi |
|------|------|------|
| Phase 1 | ✅ Selesai | Kerangka proyek, protos, errors, metadata, encoding, logging |
| Phase 2 | ✅ Selesai | Lapisan Transport (HTTP + gRPC) |
| Phase 3 | ✅ Selesai | Sistem Middleware (Recovery/Tracing/Logging/Timeout) |
| Phase 4 | ✅ Selesai | Manajemen siklus hidup App |
| Phase 5 | ✅ Selesai | Registry, Config, Metrics |
| Phase 5.5 | ✅ Selesai | Lapisan akses Data (traits + backend sqlx) |
| Phase 6 | ✅ Selesai | Rantai alat CLI (new/proto/run/build) |
| Phase 7 | ✅ Selesai | README, contoh (helloworld), dokumen desain |
| Phase 8 | ✅ Selesai | Integrasi deteksi serangan (security-rust, ecat-security) |
| Phase 9 | ✅ Selesai | Ekosistem tahap 1 (health / client / circuit-breaker / auth / registry-consul) |
| Phase 10 | ✅ Selesai | Ekosistem tahap 2 (redis / mq / events / config-remote) |
| Phase 11 | ✅ Selesai | Ekosistem tahap 3 (testing / deploy / bench / openapi) |
| Phase 12 | ✅ Selesai | Penguatan komunikasi & keamanan (klien gRPC / OAuth2 / mTLS / pelacakan terdistribusi) |
| Phase 13 | ✅ Selesai | Kelengkapan backend data (etcd / Kafka / OpenSearch / InfluxDB) |
| Phase 14 | ✅ Selesai | Operasi & pengalaman (WebSocket / manajemen versi API / Helm / CI/CD) |
| Phase 15 | ✅ Selesai | Ekstensi ekosistem v2 (Kafka asli / RabbitMQ / MQTT / NATS / MongoDB / S3 / TDengine / OTLP / kunci terdistribusi / penjadwalan / CLI watch+upgrade) |
| Phase 16 | ✅ Selesai | Penguatan pemeliharaan v2.4 (M1 MetricsLayer / M2 RetryLayer / M3 ValidateLayer / M4 CORS / U1 crate agregat ecat / U2 examples / OAuth2 token hash / pelacakan CVE) |

## Keterbatasan yang Diketahui

- **Parsing GraphQL (ecat-graphql)**: mendukung parameter kolom dan selection bersarang (`query_field`/`mutation_field` resolver kaya dapat mengakses `args`/`variables`/`selection`); masih belum mendukung alias, fragment, dan beberapa kolom tingkat atas, jangan mengeksposnya sebagai endpoint GraphQL umum.
- **Cache introspeksi OAuth2 (ecat-auth)**: key cache adalah SHA-256 hash dari token (tidak menyimpan token plaintext); nilai cache difilter whitelist (default mempertahankan sub/exp/iat/role + iss/aud/scope/roles dari extra, `cache_claims_whitelist` dapat dikonfigurasi; saat miss tetap mengembalikan claims lengkap, hanya nilai cache yang difilter); entri yang kedaluwarsa TTL dibersihkan aktif saat ditulis (TTL default 300s).
- **Kafka offset (ecat-mq-kafka)**: default `enable.auto.commit=false` dan tanpa commit manual — setelah proses restart, membaca ulang dari akhir partisi (latest), pesan yang dihasilkan selama downtime akan dilewati; perlu mengonfigurasi `auto_commit=true` secara eksplisit untuk memiliki semantik at-least-once (setelah restart berlanjut dari titik commit terbaru).

## Tujuan Desain

| # | Tujuan | Keterangan |
|---|------|------|
| 1 | **Selaras dengan Kratos** | Mempertahankan filosofi API-first, pluggable, dan abstraksi terpadu Kratos |
| 2 | **Idiomatik Rust** | Menggunakan kembali tower::Service, generik trait, abstraksi biaya nol; tidak membuat "Go in Rust" |
| 3 | **Type-safety** | Menangkap error pada waktu kompilasi, definisi Protobuf sepenuhnya bertipe kuat |
| 4 | **Pluggable** | Registry, Config, Logging, Encoding semuanya diabstraksikan melalui trait |
| 5 | **Rantai alat lengkap** | CLI mendukung scaffolding proyek, pembuatan kode proto, menjalankan pengembangan |
| 6 | **Prioritas performa** | Abstraksi biaya nol + runtime asinkron |
| 7 | **Observable** | tracing + Prometheus siap pakai |
| 8 | **Ekosistem lengkap** | Klien, circuit breaker, autentikasi, health check, backend registry |

## Catatan Teknis

### Mengapa tower::Service

[`tower::Service`](https://docs.rs/tower/latest/tower/trait.Service.html) adalah padanan `http.Handler` dalam ekosistem asinkron Rust. axum dan tonic keduanya dibangun di atas tower, sehingga e-cat tidak memerlukan trait middleware khusus — cukup menyediakan implementasi tower::Layer untuk mencapai efek yang sama dengan middleware Kratos, tanpa biaya adapter apa pun.

### Mengapa Cargo Workspace

Selaras dengan desain modular Kratos. Semua crate `ecat-*` dirilis dengan versi lockstep workspace (saat ini 3.0.2), masing-masing dikompilasi secara independen, pengguna mengimpornya sesuai kebutuhan. Crate inti mempertahankan dependensi minimal, crate contrib menyediakan integrasi opsional.

### Mengapa prost (bukan protobuf-rs)

prost adalah implementasi protobuf yang paling banyak digunakan di komunitas Rust, menghasilkan kode type-safe pada waktu kompilasi, dengan integrasi mendalam ke tonic.

## Dokumen Desain

- [Spesifikasi Desain](../../../docs/superpowers/specs/2026-07-29-ecat-framework-design.md)
- [Rencana Implementasi](../../../docs/superpowers/plans/2026-07-29-ecat-framework.md)
- [Rencana Ekosistem v1](ecosystem-plan.md) (selesai)
- [Rencana Ekosistem v2](ecosystem-plan-v2.md) (selesai)
- [Rencana Ekosistem v3](ecosystem-plan-v3.md) (evaluasi akhir)
- [Referensi API](api.md)
- [Laporan Audit r5](audit-report-2026-08-01-r5.md) (2026-08-01)
- [Tutorial Konfigurasi Database](database-config-tutorial.md)
- [Pelacakan CVE Dependensi](dependency-cve-tracking.md)
- [Tutorial Sertifikat TLS](tls-certificate-tutorial.md)
- [Contoh file konfigurasi](../../../config/databases.example.yaml)

## Dukungan

Selamat datang untuk mendukung proyek ini!

| WeChat Pay | Alipay |
|:---:|:---:|
| <img src="weixinpay.png" width="130" height="130" alt="WeChat Pay"> | <img src="alipay.png" width="130" height="130" alt="Alipay"> |

### Transfer Global (Transfer Bank)

| Item | Informasi |
|------|------|
| Nama Penerima | WANG KEXUN |
| Nomor Rekening Penerima | 881015918251 |
| Bank Penerima | ZA Bank Limited |
| SWIFT Code | AABLHKHHXXX |
| Nomor Bank | 387 |
| Alamat Bank | Core F, Cyberport 3, 100 Cyberport Road, Hong Kong |

> **Bank perantara transfer lintas negara (jika diperlukan)**: ini adalah informasi bank perantara (bank koresponden), bukan informasi bank penerima, silakan tanyakan ke bank pengirim apakah perlu disediakan.
>
> - Untuk transfer HKD, CNY, dan USD: **Citibank N.A. Hong Kong** (SWIFT: `CITIHKHXXXX`, Nomor Bank: 006, Cabang: Hong Kong Branch, Nomor Cabang: 391, Alamat: Citibank Tower, Citibank Plaza, 3 Garden Road, Central, Hong Kong)
> - Untuk mata uang lainnya: **THE BANK OF NEW YORK MELLON** (SWIFT: `IRVTUS3NXXX`, Alamat: 240 GREENWICH STREET, NEW YORK, United States)

## Lisensi

Apache-2.0
