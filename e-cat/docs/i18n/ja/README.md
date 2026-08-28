<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# Ecat

[简体中文](../../../README.md) | [English](../../../README.en.md) | **[日本語](../ja/README.md)** | [한국어](../ko/README.md) | [Русский](../ru/README.md) | [Deutsch](../de/README.md) | [Français](../fr/README.md) | [Español](../es/README.md) | [Português](../pt/README.md) | [हिन्दी](../hi/README.md) | [العربية](../ar/README.md) | [বাংলা](../bn/README.md) | [Bahasa Indonesia](../id/README.md)

Ecat の日本語名: 一匹の猫

**一匹の猫** は [go-kratos/kratos](https://github.com/go-kratos/kratos) v3 に対抗する Rust マイクロサービスフレームワークです（v3.0.2 · 51 crates）。

API-first の開発体験、プラグイン可能なコンポーネントアーキテクチャ、統一された HTTP/gRPC ミドルウェア抽象、そして充実した CLI ツールチェーンを提供します。Kratos に慣れた開発者がシームレスに使い始められる一方、Rust の型安全性、ゼロコスト抽象、極限のパフォーマンスを最大限に活用できます。

<p align="center">
  <img src="e-cat.svg" alt="Ecat プロジェクトマスコット（動的）" width="220" />
</p>

## 設計アーキテクチャ

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

### リクエスト処理フロー

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

## 機能

- **API-first**：Protobuf で API・エラーコード・メタデータを定義；prost + tonic-build によるコード生成
- **デュアルプロトコル対応**：HTTP（axum）と gRPC（tonic）が同一の tower::Layer ミドルウェアを共有
- **プラグイン可能なアーキテクチャ**：Registry、Config、Logging、Encoding をすべて trait で抽象化し、デフォルトで本番利用可能な実装を提供
- **ミドルウェア体系**：Recovery、Tracing、Logging、Timeout、RateLimit、Security、CircuitBreaker、MetricsLayer、RetryLayer、ValidateLayer、CORS（cors feature）を内蔵；tower::ServiceBuilder で組み合わせ
- **アプリケーションライフサイクル**：Builder パターンで App を構築、複数 Server の並行起動、SIGTERM/SIGINT シグナル処理、start/stop ライフサイクルフック
- **型安全性**：protobuf ベースのエラーコード体系、コンパイル時の HTTP ステータスコードマッピング
- **可観測性**：tracing + Prometheus + Health エンドポイント（/health、/ready）
- **攻撃検知**：SecurityLayer が SQL インジェクション、XSS、SSRF などの攻撃パターンを自動検出し、高危険度リクエストをブロック
- **サービス間通信**：HttpClient がサービスディスカバリとロードバランシングを統合、CircuitBreaker によるサーキットブレーカー保護
- **認証・認可**：JWT / API Key 認証ミドルウェア、Claims をリクエストコンテキストに伝搬
- **メッセージとイベント**：MessageQueue trait + EventBus によるローカル/リモート Pub/Sub
- **分散トレーシング**：リクエスト span、trace_id の注入/抽出
- **gRPC クライアント**：GrpcClient がサービスディスカバリとロードバランシングを統合
- **マルチプロトコル**：HTTP、gRPC、WebSocket、GraphQL の統一ルーティング
- **マルチデータソース**：RDBMS（SQLite/PG/MySQL/TiDB）、キャッシュ（Redis/Memcached）、検索（OpenSearch/Elasticsearch）、グラフ（Neo4j/NebulaGraph/ArangoDB）、時系列（InfluxDB/IoTDB/QuestDB/TDengine）、ドキュメント（MongoDB）、オブジェクトストレージ（S3/MinIO）

### Kratos コンセプトマッピング

| Kratos (Go) | e-cat (Rust) | 説明 |
|-------------|-------------|------|
| `kratos.New()` | `App::builder()` | Builder パターン |
| `http.Handler` | `tower::Service` | Rust エコシステム標準の trait |
| `http.Server` | `axum::Router` | コミュニティ主流の HTTP フレームワーク |
| `grpc.Server` | `tonic::transport::Server` | 最も成熟した gRPC 実装 |
| `proto generate` | `prost + tonic-build` | コミュニティ標準の protobuf |
| `registry.Discovery` | `Registry` trait | プラグイン可能な登録・ディスカバリ |
| `config.Source` | `ConfigSource` trait | マルチソース設定ロード |

## 技術スタック

| コンポーネント | 選定 |
|------|------|
| 非同期ランタイム | **tokio** |
| HTTP | **axum** |
| gRPC | **tonic** |
| Protobuf | **prost + tonic-build** |
| ミドルウェア | **tower::Service / Layer** |
| ログ/トレーシング | **tracing + trace_id propagation** |
| メトリクス | **prometheus** |
| シリアライズ | **serde + prost** |
| 攻撃検知 | **security-rust** |
| RDBMS | **sqlx** |
| Redis | **redis-rs** |
| JWT | **jsonwebtoken** |
| HTTP クライアント | **reqwest** |
| CLI | **clap** |

## 対応データベース

| カテゴリ | データベース | Crate | ステータス |
|------|--------|-------|------|
| RDBMS | SQLite | `ecat-data-sqlx` | ✅ 実装済み |
| RDBMS | PostgreSQL | `ecat-data-sqlx` | ✅ 実装済み |
| RDBMS | MySQL | `ecat-data-sqlx` | ✅ 実装済み |
| RDBMS | TiDB | `ecat-data-sqlx` | ✅ 実装済み |
| キャッシュ | Redis | `ecat-data-redis` | ✅ 実装済み |
| 検索 | OpenSearch | `ecat-data-opensearch` | ✅ 実装済み |
| 検索 | Elasticsearch | `ecat-data-elasticsearch` | ✅ 実装済み |
| キャッシュ | Memcached | `ecat-data-memcached` | ⚠️ メモリ実装（非本番用、永続キャッシュには使用しないでください） |
| OLAP | ClickHouse | `ecat-data-clickhouse` | ✅ 実装済み |
| グラフ | Neo4j | `ecat-data-neo4j` | ✅ REST API |
| グラフ | NebulaGraph | `ecat-data-nebulagraph` | ✅ REST API |
| グラフ | ArangoDB | `ecat-data-arangodb` | ✅ REST API |
| 時系列 | InfluxDB | `ecat-data-influxdb` | ✅ HTTP API |
| 時系列 | Apache IoTDB | `ecat-data-iotdb` | ✅ REST API |
| 時系列 | QuestDB | `ecat-data-questdb` | ✅ HTTP API |
| 時系列 | TDengine | `ecat-data-tdengine` | ✅ REST API |
| ドキュメント | MongoDB | `ecat-data-mongodb` | ✅ ネイティブドライバ |
| オブジェクトストレージ | S3 / MinIO | `ecat-data-s3` | ✅ reqwest+rustls |

> すべてのデータバックエンドは統一 trait 抽象（`RdbmsClient` / `Cache` / `SearchClient` / `GraphClient` / `TsdbClient` / `DocumentClient` / `StorageClient`）を通じて利用でき、必要に応じて対応する contrib crate を導入します。各バックエンドは `XxxConfig` 構造体（`#[derive(Deserialize)]`）を提供し、JSON/YAML 設定ファイルから接続情報をロードできます。

> **コンストラクタ命名規約**：メッセージキュー crate（`ecat-mq-*`）の主コンストラクタは統一して `connect`（例：`KafkaMq::connect(brokers)`、`MqttMq::connect(url)`）、その他に設定からロードする `from_config` を提供；データバックエンド crate（`ecat-data-*`）の主コンストラクタは大半が `new`。例外：`ecat-data-redis` / `ecat-data-sqlx` は `connect` を踏襲、`ecat-data-mongodb` / `ecat-data-s3` は `from_config` のみ提供。これは既存の規約であり、強制はしません（破壊的変更を避けるため）；3.0 の窓口で統一を評価可能です。

### データベース設定例

各データバックエンドは `XxxConfig` 構造体と `from_config()` メソッドを提供し、接続情報をコードから設定ファイルへ分離します：

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

**設定フィールド参考**:

| バックエンド | Config | フィールド | サンプル値 |
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
| Memcached | `MemcachedConfig` | `username`?, `password`?（保留フィールド） | — |
| TDengine | `TdengineConfig` | `base_url`, `username`, `password`, `database`? | `http://localhost:6041` |
| MongoDB | `MongoConfig` | `url`, `database`, `tls`? | `mongodb://localhost:27017`, `app` |
| S3 | `S3Config` | `endpoint`, `region`, `access_key`, `secret_key`, `tls`? | `http://localhost:9000`, `us-east-1` |

> すべてのバックエンド Config はオプションの `tls` フィールド（`TlsClientConfig`）をサポートし、TLS クライアント証明書認証を設定できます。詳細は [データベース設定チュートリアル](database-config-tutorial.md) を参照してください。

## プロジェクト構造

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

## クイックスタート

### 前提条件

- Rust 1.85+（stable ツールチェーン、edition 2024 必須）
- [protoc](https://github.com/protocolbuffers/protobuf)（Protocol Buffers コンパイラ）

### CLI のインストール

```bash
cargo install ecat-cli
```

### サービスの作成

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

`http://localhost:8000/helloworld/ecat` にアクセスしてください。

### コード例

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

### 集約 crate（ecat）

`ecat` は feature-gated の re-export エントリを提供します — 必要なコンポーネントだけを有効にできます：

```rust
use ecat::transport_http::HttpServer;   // feature "http"（默认）
use ecat::middleware::RecoveryLayer;     // feature "middleware"
use ecat::auth::JwtAuthLayer;            // feature "auth"
use ecat::data::redis::RedisCache;       // feature "redis"
```

デフォルト features = `http+grpc`；`--no-default-features --features <コンポーネント>` で依存ツリーを削減できます。完全な feature リスト：`http` `grpc` `middleware` `auth` `client` `events` `metrics` `tracing` `circuit-breaker` `consul` `remote` `redis`。

### ミドルウェア

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

> 注：`ecat_middleware::TracingLayer` は trace_id を注入しません。リクエストレベルの trace_id 注入が必要な場合は `ecat_tracing::TracingLayer::new()` を使用してください。

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

### エラーハンドリング

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

## 実装フェーズ

| フェーズ | ステータス | 内容 |
|------|------|------|
| Phase 1 | ✅ 完了 | プロジェクト骨格、protos、errors、metadata、encoding、logging |
| Phase 2 | ✅ 完了 | Transport 層（HTTP + gRPC） |
| Phase 3 | ✅ 完了 | Middleware 体系（Recovery/Tracing/Logging/Timeout） |
| Phase 4 | ✅ 完了 | App ライフサイクル管理 |
| Phase 5 | ✅ 完了 | Registry、Config、Metrics |
| Phase 5.5 | ✅ 完了 | Data アクセス層（traits + sqlx バックエンド） |
| Phase 6 | ✅ 完了 | CLI ツールチェーン（new/proto/run/build） |
| Phase 7 | ✅ 完了 | README、サンプル（helloworld）、設計ドキュメント |
| Phase 8 | ✅ 完了 | 攻撃検知統合（security-rust, ecat-security） |
| Phase 9 | ✅ 完了 | エコシステム一期（health / client / circuit-breaker / auth / registry-consul） |
| Phase 10 | ✅ 完了 | エコシステム二期（redis / mq / events / config-remote） |
| Phase 11 | ✅ 完了 | エコシステム三期（testing / deploy / bench / openapi） |
| Phase 12 | ✅ 完了 | 通信とセキュリティ強化（gRPC クライアント / OAuth2 / mTLS / 分散トレーシング） |
| Phase 13 | ✅ 完了 | データバックエンド補充（etcd / Kafka / OpenSearch / InfluxDB） |
| Phase 14 | ✅ 完了 | 運用と体験（WebSocket / API バージョン管理 / Helm / CI/CD） |
| Phase 15 | ✅ 完了 | エコシステム拡張 v2（本物の Kafka / RabbitMQ / MQTT / NATS / MongoDB / S3 / TDengine / OTLP / 分散ロック / スケジューラ / CLI watch+upgrade） |
| Phase 16 | ✅ 完了 | 保守強化 v2.4（M1 MetricsLayer / M2 RetryLayer / M3 ValidateLayer / M4 CORS / U1 集約 crate ecat / U2 examples / OAuth2 token hash / CVE トラッキング） |

## 既知の制限

- **GraphQL パーシング（ecat-graphql）**：フィールドパラメータとネスト selection をサポート（`query_field`/`mutation_field` のリッチ resolver が `args`/`variables`/`selection` にアクセス可能）；エイリアス、fragment、複数トップレベルフィールドは未対応のため、汎用 GraphQL エンドポイントとして公開しないでください。
- **OAuth2 イントロスペクションキャッシュ（ecat-auth）**：キャッシュキーは token の SHA-256 ハッシュ（token 平文は保存しない）；キャッシュ値はホワイトリストでフィルタリング（デフォルトで sub/exp/iat/role + extra の iss/aud/scope/roles を保持、`cache_claims_whitelist` で設定可能；miss 時は完全な claims を返し、キャッシュ値のみフィルタリング）；TTL 期限切れエントリは書き込み時に積極的に削除（デフォルト TTL 300s）。
- **Kafka offset（ecat-mq-kafka）**：デフォルトは `enable.auto.commit=false` で手動 commit なし — プロセス再起動後はパーティション末尾（latest）から読み直すため、停止期間中に生成されたメッセージはスキップされます；`auto_commit=true` を明示的に設定して初めて at-least-once セマンティクスになります（再起動時は最後のコミット位置から継続）。

## 設計目標

| # | 目標 | 説明 |
|---|------|------|
| 1 | **Kratos アライメント** | Kratos の API-first、プラグイン可能、統一抽象の理念を維持 |
| 2 | **Rust 慣用** | tower::Service、trait ジェネリクス、ゼロコスト抽象を再利用；「Go in Rust」はしない |
| 3 | **型安全性** | コンパイル時にエラーを捕捉、Protobuf 定義を全強型付け |
| 4 | **プラグイン可能** | Registry、Config、Logging、Encoding をすべて trait で抽象化 |
| 5 | **ツールチェーン完備** | CLI がプロジェクトスキャフォールド、proto コード生成、開発実行をサポート |
| 6 | **パフォーマンス優先** | ゼロコスト抽象 + 非同期ランタイム |
| 7 | **可観測** | tracing + Prometheus がすぐに使える |
| 8 | **エコシステム完備** | クライアント、サーキットブレーカー、認証、ヘルスチェック、レジストリバックエンド |

## 技術解説

### なぜ tower::Service か

[`tower::Service`](https://docs.rs/tower/latest/tower/trait.Service.html) は Rust 非同期エコシステムにおける `http.Handler` の等価物です。axum と tonic はどちらも tower 上に構築されているため、e-cat は独自のミドルウェア trait を必要としません — tower::Layer 実装を提供するだけで Kratos のミドルウェアと同じ効果が得られ、アダプタのオーバーヘッドもゼロです。

### なぜ Cargo Workspace か

Kratos のモジュール設計と一致しています。すべての `ecat-*` crate は workspace でロックステップのバージョン（現在 3.0.2）でリリースされ、それぞれ独立してコンパイルされ、ユーザーが必要に応じて導入します。コア crate は最小限の依存関係を維持し、contrib crate がオプションの統合を提供します。

### なぜ prost（protobuf-rs ではなく）か

prost は Rust コミュニティで最も広く使われている protobuf 実装であり、コンパイル時に型安全なコードを生成し、tonic と深く統合されています。

## 設計ドキュメント

- [設計仕様](../../../docs/superpowers/specs/2026-07-29-ecat-framework-design.md)
- [実装計画](../../../docs/superpowers/plans/2026-07-29-ecat-framework.md)
- [エコシステム計画 v1](ecosystem-plan.md)（完了）
- [エコシステム計画 v2](ecosystem-plan-v2.md)（完了）
- [エコシステム計画 v3](ecosystem-plan-v3.md)（最終評価）
- [API リファレンス](api.md)
- [監査レポート r5](audit-report-2026-08-01-r5.md)（2026-08-01）
- [データベース設定チュートリアル](database-config-tutorial.md)
- [依存関係 CVE トラッキング](dependency-cve-tracking.md)
- [TLS 証明書認証チュートリアル](tls-certificate-tutorial.md)
- [設定サンプルファイル](../../../config/databases.example.yaml)

## サポート

本プロジェクトへのサポートを歓迎します！

| 微信支付（WeChat Pay） | 支付宝（Alipay） |
|:---:|:---:|
| <img src="weixinpay.png" width="130" height="130" alt="微信支付（WeChat Pay）"> | <img src="alipay.png" width="130" height="130" alt="支付宝（Alipay）"> |

### グローバル送金（銀行振込）

| 項目 | 情報 |
|------|------|
| 受取人氏名 | WANG KEXUN |
| 受取口座番号 | 881015918251 |
| 受取銀行 | ZA Bank Limited |
| SWIFT Code | AABLHKHHXXX |
| 銀行コード | 387 |
| 銀行住所 | Core F, Cyberport 3, 100 Cyberport Road, Hong Kong |

> **クロスボーダー送金の代理銀行（必要な場合）**：これは代理銀行（中継銀行）の情報であり、受取銀行の情報ではありません。送金銀行に必要かどうかお問い合わせください。
>
> - 香港ドル・人民元・米ドルの送金：**Citibank N.A. Hong Kong**（SWIFT：`CITIHKHXXXX`、銀行コード：006、支店：Hong Kong Branch、支店コード：391、住所：Citibank Tower, Citibank Plaza, 3 Garden Road, Central, Hong Kong）
> - その他通貨の送金：**THE BANK OF NEW YORK MELLON**（SWIFT：`IRVTUS3NXXX`、住所：240 GREENWICH STREET, NEW YORK, United States）

## ライセンス

Apache-2.0
