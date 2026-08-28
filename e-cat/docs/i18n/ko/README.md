<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# Ecat

[简体中文](../../../README.md) | [English](../../../README.en.md) | [日本語](../ja/README.md) | **한국어** | [Русский](../ru/README.md) | [Deutsch](../de/README.md) | [Français](../fr/README.md) | [Español](../es/README.md) | [Português](../pt/README.md) | [हिन्दी](../hi/README.md) | [العربية](../ar/README.md) | [বাংলা](../bn/README.md) | [Bahasa Indonesia](../id/README.md)

Ecat 한국어 이름: 한 마리 고양이

**한 마리 고양이**는 [go-kratos/kratos](https://github.com/go-kratos/kratos) v3를 벤치마킹한 Rust 마이크로서비스 프레임워크입니다 (v3.0.2 · 51 crates).

API-first 개발 경험, 플러그 가능한 컴포넌트 아키텍처, 통합된 HTTP/gRPC 미들웨어 추상화, 그리고 완비된 CLI 도구 체인을 제공합니다. Kratos에 익숙한 개발자가 매끄럽게 적응할 수 있으면서도, Rust의 타입 안전성, 제로 비용 추상화, 극한의 성능을 충분히 활용할 수 있습니다.

<p align="center">
  <img src="e-cat.svg" alt="Ecat 프로젝트 펫(동적)" width="220" />
</p>

## 설계 아키텍처

```
┌──────────────────────────────────────────────────────────────┐
│                         ecat-cli                             │
│        (new │ proto │ run --watch │ build │ upgrade)         │
├──────────────────────────────────────────────────────────────┤
│                     ecat (애플리케이션 수명주기)                │
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
│                         data 계층                            │
│     ────────────────────────────────────────────────          │
│     rdbms:   SQLite / PostgreSQL / MySQL / TiDB              │
│     cache:   Redis ✓                                         │
│     config:  remote (Consul KV)                              │
│     registry: consul                                         │
├──────────────────────────────────────────────────────────────┤
│                       ecat-protos                             │
│     (공유 .proto 정의: errors, metadata, ...)                 │
└──────────────────────────────────────────────────────────────┘
```

### 요청 처리 흐름

```
클라이언트 요청
  │
  ├─ HTTP 0.0.0.0:8000 ──→ axum::Router ──┐
  │                                        │
  └─ gRPC 0.0.0.0:9000 ──→ tonic::Server ─┤
                                      │
                              ┌───────┴───────┐
                              │   Middleware   │
                              │   ──────────   │
                              │ 1. Recovery    │  panic 포착
                              │ 2. Tracing     │  trace_id 주입
                              │ 3. Logging     │  요청 로그
                              │ 4. Auth        │  인증·권한 부여
                              │ 5. Metrics     │  메트릭 수집
│ 6. Security    │  공격 탐지
│ 7. CircuitBrk  │  회로 차단 보호
                              └───────┬───────┘
                                      │
                              ┌───────┴───────┐
                              │    Handler     │  사용자 비즈니스 로직
                              │ (tower::Service)│
                              └───────┬───────┘
                                      │
                              ┌───────┴───────┐
                              │   Response     │  인코딩·직렬화
                              │ JSON/Protobuf  │
                              └───────────────┘
```

## 기능

- **API-first**: Protobuf로 API·오류 코드·메타데이터 정의; prost + tonic-build 코드 생성
- **이중 프로토콜 지원**: HTTP(axum)와 gRPC(tonic)가 동일한 tower::Layer 미들웨어 공유
- **플러그 가능한 아키텍처**: Registry, Config, Logging, Encoding 모두 trait으로 추상화, 기본 제공되는 프로덕션 준비 구현
- **미들웨어 체계**: 내장 Recovery, Tracing, Logging, Timeout, RateLimit, Security, CircuitBreaker, MetricsLayer, RetryLayer, ValidateLayer, CORS(cors feature); tower::ServiceBuilder로 조합
- **애플리케이션 수명주기**: Builder 패턴으로 App 구성, 다중 Server 동시 기동, SIGTERM/SIGINT 시그널 처리, start/stop 수명주기 훅
- **타입 안전성**: protobuf 기반 오류 코드 체계, 컴파일 타임 HTTP 상태 코드 매핑
- **관측성**: tracing + Prometheus + Health 엔드포인트(/health, /ready)
- **공격 탐지**: SecurityLayer가 SQL 인젝션, XSS, SSRF 등 공격 패턴 자동 탐지, 고위험 요청 차단
- **서비스 간 통신**: HttpClient가 서비스 디스커버리·로드 밸런싱 통합, CircuitBreaker 회로 차단 보호
- **인증·권한 부여**: JWT / API Key 인증 미들웨어, Claims를 요청 컨텍스트로 전달
- **메시지와 이벤트**: MessageQueue trait + EventBus 로컬/원격 Pub/Sub
- **분산 추적**: 요청 span, trace_id 주입/추출
- **gRPC 클라이언트**: GrpcClient가 서비스 디스커버리·로드 밸런싱 통합
- **다중 프로토콜**: HTTP, gRPC, WebSocket, GraphQL 통합 라우팅
- **다중 데이터 소스**: RDBMS(SQLite/PG/MySQL/TiDB), 캐시(Redis/Memcached), 검색(OpenSearch/Elasticsearch), 그래프(Neo4j/NebulaGraph/ArangoDB), 시계열(InfluxDB/IoTDB/QuestDB/TDengine), 문서(MongoDB), 객체 스토리지(S3/MinIO)

### Kratos 개념 매핑

| Kratos (Go) | e-cat (Rust) | 설명 |
|-------------|-------------|------|
| `kratos.New()` | `App::builder()` | Builder 패턴 |
| `http.Handler` | `tower::Service` | Rust 생태계 표준 trait |
| `http.Server` | `axum::Router` | 커뮤니티 주류 HTTP 프레임워크 |
| `grpc.Server` | `tonic::transport::Server` | 가장 성숙한 gRPC 구현 |
| `proto generate` | `prost + tonic-build` | 커뮤니티 표준 protobuf |
| `registry.Discovery` | `Registry` trait | 플러그 가능한 등록·디스커버리 |
| `config.Source` | `ConfigSource` trait | 다중 소스 설정 로딩 |

## 기술 스택

| 구성 요소 | 선정 |
|------|------|
| 비동기 런타임 | **tokio** |
| HTTP | **axum** |
| gRPC | **tonic** |
| Protobuf | **prost + tonic-build** |
| 미들웨어 | **tower::Service / Layer** |
| 로그/추적 | **tracing + trace_id propagation** |
| 메트릭 | **prometheus** |
| 직렬화 | **serde + prost** |
| 공격 탐지 | **security-rust** |
| RDBMS | **sqlx** |
| Redis | **redis-rs** |
| JWT | **jsonwebtoken** |
| HTTP Client | **reqwest** |
| CLI | **clap** |

## 지원 데이터베이스

| 카테고리 | 데이터베이스 | Crate | 상태 |
|------|--------|-------|------|
| RDBMS | SQLite | `ecat-data-sqlx` | ✅ 구현됨 |
| RDBMS | PostgreSQL | `ecat-data-sqlx` | ✅ 구현됨 |
| RDBMS | MySQL | `ecat-data-sqlx` | ✅ 구현됨 |
| RDBMS | TiDB | `ecat-data-sqlx` | ✅ 구현됨 |
| 캐시 | Redis | `ecat-data-redis` | ✅ 구현됨 |
| 검색 | OpenSearch | `ecat-data-opensearch` | ✅ 구현됨 |
| 검색 | Elasticsearch | `ecat-data-elasticsearch` | ✅ 구현됨 |
| 캐시 | Memcached | `ecat-data-memcached` | ⚠️ 메모리 구현(비프로덕션, 영구 캐시로 사용 금지) |
| OLAP | ClickHouse | `ecat-data-clickhouse` | ✅ 구현됨 |
| 그래프 | Neo4j | `ecat-data-neo4j` | ✅ REST API |
| 그래프 | NebulaGraph | `ecat-data-nebulagraph` | ✅ REST API |
| 그래프 | ArangoDB | `ecat-data-arangodb` | ✅ REST API |
| 시계열 | InfluxDB | `ecat-data-influxdb` | ✅ HTTP API |
| 시계열 | Apache IoTDB | `ecat-data-iotdb` | ✅ REST API |
| 시계열 | QuestDB | `ecat-data-questdb` | ✅ HTTP API |
| 시계열 | TDengine | `ecat-data-tdengine` | ✅ REST API |
| 문서 | MongoDB | `ecat-data-mongodb` | ✅ 네이티브 드라이버 |
| 객체 스토리지 | S3 / MinIO | `ecat-data-s3` | ✅ reqwest+rustls |

> 모든 데이터 백엔드는 통일된 trait 추상화(`RdbmsClient` / `Cache` / `SearchClient` / `GraphClient` / `TsdbClient` / `DocumentClient` / `StorageClient`)를 통해 제공되며, 필요에 따라 해당 contrib crate를 가져와 사용합니다. 각 백엔드는 `XxxConfig` 구조체(`#[derive(Deserialize)]`)를 제공하여 JSON/YAML 설정 파일에서 연결 정보를 로드할 수 있습니다.

> **생성자 명명 규칙**: 메시지 큐 crate(`ecat-mq-*`)의 주 생성자는 통일적으로 `connect`(`KafkaMq::connect(brokers)`, `MqttMq::connect(url)` 등)이며, 별도로 `from_config`로 설정에서 로드할 수 있습니다; 데이터 백엔드 crate(`ecat-data-*`)는 대부분 `new`가 주 생성자이며, 예외: `ecat-data-redis` / `ecat-data-sqlx`는 `connect`를 유지하고, `ecat-data-mongodb` / `ecat-data-s3`는 `from_config`만 제공합니다. 이는 기존 규약이며 강제로 통일하지 않습니다(파괴적 변경 방지); 3.0 창구에서 통일을 평가할 수 있습니다.

### 데이터베이스 설정 예시

각 데이터 백엔드는 `XxxConfig` 구조체와 `from_config()` 메서드를 제공하여 연결 정보를 코드에서 설정 파일로 분리합니다:

```rust
use ecat_data_redis::{RedisCache, RedisConfig};
use ecat_data_sqlx::{SqlxClient, SqlxConfig};
use ecat_data_clickhouse::{ClickhouseClient, ClickhouseConfig};

// 설정 파일에서 로드 (JSON 또는 YAML)
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

**설정 필드 참조**:

| 백엔드 | Config | 필드 | 예시 값 |
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
| Memcached | `MemcachedConfig` | `username`?, `password`?(예약 필드) | — |
| TDengine | `TdengineConfig` | `base_url`, `username`, `password`, `database`? | `http://localhost:6041` |
| MongoDB | `MongoConfig` | `url`, `database`, `tls`? | `mongodb://localhost:27017`, `app` |
| S3 | `S3Config` | `endpoint`, `region`, `access_key`, `secret_key`, `tls`? | `http://localhost:9000`, `us-east-1` |

> 모든 백엔드 Config는 선택적 `tls` 필드(`TlsClientConfig`)를 지원하며, TLS 클라이언트 인증서 인증을 구성할 수 있습니다. 자세한 내용은 [데이터베이스 설정 튜토리얼](database-config-tutorial.md)을 참조하세요.

## 프로젝트 구조

```
e-cat/
├── ecat/                       # 핵심: App 수명주기
├── ecat-transport/             # 전송 추상화 (Server trait)
├── ecat-transport-http/        # axum 구현
├── ecat-transport-grpc/        # tonic 구현
├── ecat-middleware/            # tower::Layer 미들웨어
├── ecat-protos/                # Protobuf 정의
├── ecat-errors/                # 오류 코드 체계
├── ecat-metadata/              # 메타데이터 전달
├── ecat-encoding/              # 직렬화 추상화
├── ecat-logging/               # tracing 통합
├── ecat-registry/              # 서비스 등록·디스커버리
├── ecat-config/                # 설정 관리
├── ecat-metrics/               # Prometheus 통합
├── ecat-data/                  # 데이터 접근 trait
├── ecat-security/              # 공격 탐지 (security-rust)
├── ecat-cli/                   # CLI 도구
├── ecat-health/                # 헬스 체크 (/health /ready)
├── ecat-auth/                  # 인증 미들웨어 (JWT / API Key)
├── ecat-client/                # 서비스 간 HTTP 클라이언트
├── ecat-circuit-breaker/       # 회로 차단기 (Tower Layer)
├── ecat-registry-consul/       # Consul 서비스 등록
├── ecat-config-remote/         # Consul KV 원격 설정
├── ecat-data-redis/            # Redis 캐시 구현
├── ecat-mq/                    # 메시지 큐 추상화
├── ecat-events/                # 이벤트 버스 (로컬 + 원격)
├── ecat-testing/               # 통합 테스트 도구
├── ecat-openapi/               # OpenAPI spec 생성
├── ecat-bench/                 # 성능 벤치마크
├── ecat-tracing/               # 분산 추적 (trace_id 주입/추출)
├── ecat-registry-etcd/         # etcd 서비스 등록
├── ecat-mq-kafka/              # Kafka 메시지 큐 어댑터
├── ecat-data-opensearch/       # OpenSearch 검색 백엔드
├── ecat-data-influxdb/         # InfluxDB 시계열 백엔드
├── ecat-graphql/               # GraphQL endpoint
├── ecat-data-elasticsearch/    # Elasticsearch 검색 백엔드
├── ecat-data-clickhouse/       # ClickHouse OLAP 백엔드
├── ecat-data-sqlx/             # RDBMS 백엔드 (SQLite/PG/MySQL/TiDB)
├── ecat-data-memcached/        # Memcached 캐시 백엔드 (메모리 구현)
├── ecat-data-neo4j/            # Neo4j 그래프 백엔드
├── ecat-data-nebulagraph/      # NebulaGraph 그래프 백엔드
├── ecat-data-arangodb/         # ArangoDB 그래프 백엔드
├── ecat-data-iotdb/            # IoTDB 시계열 백엔드
├── ecat-data-questdb/          # QuestDB 시계열 백엔드
├── ecat-transport-ws/          # WebSocket transport
├── ecat-versioning/            # API 버전 라우팅
├── ecat-tls/                   # TLS 인증서 설정과 자동 생성
├── ecat-deploy/                # Docker / K8s / Helm / CI/CD
├── ecat-lock/                  # 분산 잠금 추상화 (Redis 구현)
├── ecat-scheduler/             # tokio 정기 작업 스케줄링
├── ecat-tracing-otlp/          # OpenTelemetry OTLP 추적 내보내기
├── ecat-data-tdengine/         # TDengine 시계열 백엔드
├── ecat-data-mongodb/          # MongoDB 문서 백엔드
├── ecat-data-s3/               # S3 / MinIO 객체 스토리지 백엔드
├── ecat-mq-rabbitmq/           # RabbitMQ 메시지 백엔드
├── ecat-mq-mqtt/               # MQTT 메시지 백엔드
├── ecat-mq-nats/               # NATS 메시지 백엔드
├── config/                     # 설정 예시 파일
├── docs/                       # 설계 문서와 생태계 계획
└── examples/                   # 예시 프로젝트
```

## 빠른 시작

### 사전 요구 사항

- Rust 1.85+ (stable 툴체인, edition 2024 요구)
- [protoc](https://github.com/protocolbuffers/protobuf) (Protocol Buffers 컴파일러)

### CLI 설치

```bash
cargo install ecat-cli
```

### 서비스 생성

```bash
# 스캐폴딩으로 프로젝트 생성
ecat new helloworld
cd helloworld

# proto 정의 추가
ecat proto add proto/service.proto

# 클라이언트·서버 코드 생성 (tonic-build build.rs, Cargo.toml 의존성 자동 보완)
ecat proto client proto/service.proto
ecat proto server proto/service.proto -t internal/service

# 개발 모드 실행
ecat run

# src/ 변경 감지 자동 재시작
ecat run --watch

# 모든 ecat-* 의존성 업데이트
ecat upgrade
```

`http://localhost:8000/helloworld/ecat`에 접속하세요.

### 코드 예시

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

    app.run().await?; // SIGTERM/SIGINT 수신까지 블로킹
    Ok(())
}
```

### 집계 crate (ecat)

`ecat`는 feature-gated re-export 진입점을 제공합니다 — 필요한 컴포넌트만 활성화하세요:

```rust
use ecat::transport_http::HttpServer;   // feature "http" (기본)
use ecat::middleware::RecoveryLayer;     // feature "middleware"
use ecat::auth::JwtAuthLayer;            // feature "auth"
use ecat::data::redis::RedisCache;       // feature "redis"
```

기본 features = `http+grpc`; `--no-default-features --features <컴포넌트>`로 의존성 트리를 간소화할 수 있습니다. 전체 feature 목록: `http` `grpc` `middleware` `auth` `client` `events` `metrics` `tracing` `circuit-breaker` `consul` `remote` `redis`.

### 미들웨어

```rust
use tower::ServiceBuilder;
use ecat_middleware::{RecoveryLayer, TracingLayer, LoggingLayer, TimeoutLayer};
use ecat_circuit_breaker::CircuitBreakerLayer;
use ecat_security::SecurityLayer;
use ecat_auth::JwtAuthLayer;
use std::time::Duration;

// JWT 키는 ≥32바이트 필요; 체이닝으로 iss/aud 클레임 강제 검증 가능 (선택, 기본 검증 안 함):
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

> 참고: `ecat_middleware::TracingLayer`는 trace_id를 주입하지 않습니다. 요청 단위 trace_id 주입이 필요하면 `ecat_tracing::TracingLayer::new()`를 사용하세요.

```rust
// 메트릭: 요청 횟수·지연을 전역 registry에 기록 (/metrics 엔드포인트와 공유)
use ecat_metrics::MetricsLayer;
let app = Router::new().route("/hello", get(hello)).layer(MetricsLayer::new());
// 메트릭 이름: ecat_http_requests_total / ecat_http_request_duration_seconds
// (라벨 method/path/status). 경로에 ID 등 고기수(高基数) 시나리오는
// MetricsLayer::new().with_path_fn(...)으로 정규화하여 메트릭 카디널리티 폭발을 방지하세요.

// 재시도: 지수 백오프; ⚠️ 멱등 요청(GET/HEAD/PUT/DELETE)에만 안전
use ecat_middleware::RetryLayer;
let retry = RetryLayer::new(3, Duration::from_secs(1), Duration::from_secs(30)); // 첫 시도 포함 총 3회
// 사용자 정의 재시도 규칙: RetryLayer::new(3, ...).with_rule(MyRule)  // 상태 코드/응답 내용으로 판정

// 검증: 라우팅 전에 header/파라미터 검증, 실패 시 JSON 오류로 단락 반환 (기본 400, with_status로 422 등 변경 가능)
use ecat_middleware::{ValidateLayer, ValidateError};
let validate = ValidateLayer::from_fn(|req: &http::Request<axum::body::Body>| {
    if req.headers().contains_key("x-api-key") {
        Ok(())
    } else {
        Err(ValidateError::new("missing x-api-key").with_status(422))
    }
});

// CORS: ecat-middleware에서 "cors" feature 활성화 필요
use ecat_middleware::{CorsLayer, AllowOrigin};
let cors = CorsLayer::new().allow_origin(AllowOrigin::any());
```

### 오류 처리

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

## 구현 단계

| 단계 | 상태 | 내용 |
|------|------|------|
| Phase 1 | ✅ 완료 | 프로젝트 골격, protos, errors, metadata, encoding, logging |
| Phase 2 | ✅ 완료 | Transport 계층 (HTTP + gRPC) |
| Phase 3 | ✅ 완료 | Middleware 체계 (Recovery/Tracing/Logging/Timeout) |
| Phase 4 | ✅ 완료 | App 수명주기 관리 |
| Phase 5 | ✅ 완료 | Registry, Config, Metrics |
| Phase 5.5 | ✅ 완료 | Data 접근 계층 (traits + sqlx 백엔드) |
| Phase 6 | ✅ 완료 | CLI 도구 체인 (new/proto/run/build) |
| Phase 7 | ✅ 완료 | README, 예시(helloworld), 설계 문서 |
| Phase 8 | ✅ 완료 | 공격 탐지 통합 (security-rust, ecat-security) |
| Phase 9 | ✅ 완료 | 생태계 1기 (health / client / circuit-breaker / auth / registry-consul) |
| Phase 10 | ✅ 완료 | 생태계 2기 (redis / mq / events / config-remote) |
| Phase 11 | ✅ 완료 | 생태계 3기 (testing / deploy / bench / openapi) |
| Phase 12 | ✅ 완료 | 통신·보안 강화 (gRPC 클라이언트 / OAuth2 / mTLS / 분산 추적) |
| Phase 13 | ✅ 완료 | 데이터 백엔드 보완 (etcd / Kafka / OpenSearch / InfluxDB) |
| Phase 14 | ✅ 완료 | 운영·경험 (WebSocket / API 버전 관리 / Helm / CI/CD) |
| Phase 15 | ✅ 완료 | 생태계 확장 v2 (실제 Kafka / RabbitMQ / MQTT / NATS / MongoDB / S3 / TDengine / OTLP / 분산 잠금 / 스케줄링 / CLI watch+upgrade) |
| Phase 16 | ✅ 완료 | 유지보수 강화 v2.4 (M1 MetricsLayer / M2 RetryLayer / M3 ValidateLayer / M4 CORS / U1 집계 crate ecat / U2 examples / OAuth2 token hash / CVE 추적) |

## 알려진 제한 사항

- **GraphQL 파싱 (ecat-graphql)**: 필드 파라미터와 중첩 selection을 지원합니다 (`query_field`/`mutation_field` 리치 resolver가 `args`/`variables`/`selection`에 접근 가능). 별칭, fragment, 다중 최상위 필드는 아직 지원하지 않으므로 범용 GraphQL 엔드포인트로 노출하지 마세요.
- **OAuth2 인트로스펙션 캐시 (ecat-auth)**: 캐시 키는 token의 SHA-256 해시(token 평문 미저장); 캐시 값은 화이트리스트로 필터링(기본 sub/exp/iat/role + extra의 iss/aud/scope/roles 보존, `cache_claims_whitelist`로 설정 가능; miss 시에도 전체 claims 반환, 캐시 값만 필터링); TTL 만료 항목은 쓰기 시 능동 정리(기본 TTL 300s).
- **Kafka offset (ecat-mq-kafka)**: 기본 `enable.auto.commit=false`이며 수동 commit이 없음 — 프로세스 재시작 시 파티션 끝(latest)부터 재읽기하여 중단 기간 동안 생성된 메시지가 건너뛰어집니다; `auto_commit=true`를 명시적으로 설정해야 at-least-once 의미론을 얻습니다(재시작 시 마지막 커밋 지점부터 계속).

## 설계 목표

| # | 목표 | 설명 |
|---|------|------|
| 1 | **Kratos 정렬** | Kratos의 API-first, 플러그 가능, 통일 추상화 이념 유지 |
| 2 | **Rust 관용적** | tower::Service, trait 제네릭, 제로 비용 추상화 재사용; "Go in Rust" 금지 |
| 3 | **타입 안전** | 컴파일 타임 오류 포착, Protobuf 정의 전체 강타입화 |
| 4 | **플러그 가능** | Registry, Config, Logging, Encoding 모두 trait으로 추상화 |
| 5 | **도구 체인 완비** | CLI가 프로젝트 스캐폴딩, proto 코드 생성, 개발 실행 지원 |
| 6 | **성능 우선** | 제로 비용 추상화 + 비동기 런타임 |
| 7 | **관측 가능** | tracing + Prometheus 기본 제공 |
| 8 | **생태계 완비** | 클라이언트, 회로 차단, 인증, 헬스 체크, 등록 센터 백엔드 |

## 기술 설명

### 왜 tower::Service인가

[`tower::Service`](https://docs.rs/tower/latest/tower/trait.Service.html)는 Rust 비동기 생태계의 `http.Handler` 등가물입니다. axum과 tonic 모두 tower 위에 구축되므로, e-cat은 커스텀 미들웨어 trait이 필요 없습니다 — tower::Layer 구현만 제공하면 Kratos 미들웨어와 동일한 효과를 어댑터 오버헤드 없이 얻을 수 있습니다.

### 왜 Cargo Workspace인가

Kratos의 모듈식 설계와 일치합니다. 모든 `ecat-*` crate는 workspace 잠금 단계로 함께 버전 출시(현재 3.0.2)되며, 각자 독립 컴파일되고 사용자가 필요에 따라 가져옵니다. 핵심 crate는 최소 의존성을 유지하고, contrib crate는 선택적 통합을 제공합니다.

### 왜 prost (protobuf-rs 아님)인가

prost는 Rust 커뮤니티에서 가장 널리 사용되는 protobuf 구현으로, 컴파일 타임에 타입 안전 코드를 생성하며 tonic과 깊게 통합됩니다.

## 설계 문서

- [설계 규격](../../../docs/superpowers/specs/2026-07-29-ecat-framework-design.md)
- [구현 계획](../../../docs/superpowers/plans/2026-07-29-ecat-framework.md)
- [생태계 계획 v1](ecosystem-plan.md)(완료)
- [생태계 계획 v2](ecosystem-plan-v2.md)(완료)
- [생태계 계획 v3](ecosystem-plan-v3.md)(최종 평가)
- [API 참조](api.md)
- [감사 보고서 r5](audit-report-2026-08-01-r5.md)(2026-08-01)
- [데이터베이스 설정 튜토리얼](database-config-tutorial.md)
- [의존성 CVE 추적](dependency-cve-tracking.md)
- [TLS 인증서 인증 튜토리얼](tls-certificate-tutorial.md)
- [설정 예시 파일](../../../config/databases.example.yaml)

## 후원

이 프로젝트를 후원해 주세요!

| 위챗페이(WeChat Pay) | 알리페이(Alipay) |
|:---:|:---:|
| <img src="weixinpay.png" width="130" height="130" alt="위챗페이"> | <img src="alipay.png" width="130" height="130" alt="알리페이"> |

### 글로벌 송금 (은행 송금)

| 항목 | 정보 |
|------|------|
| 수취인 이름 | WANG KEXUN |
| 수취인 계좌 번호 | 881015918251 |
| 수취 은행 | ZA Bank Limited |
| SWIFT Code | AABLHKHHXXX |
| 은행 번호 | 387 |
| 은행 주소 | Core F, Cyberport 3, 100 Cyberport Road, Hong Kong |

> **해외 송금 대리 은행(필요 시)**: 이 정보는 대리 은행(중계 은행) 정보이며 수취 은행 정보가 아닙니다. 송금 은행에 필요한지 문의하세요.
>
> - 홍콩 달러, 위안화 및 미국 달러 송금: **Citibank N.A. Hong Kong**(SWIFT: `CITIHKHXXXX`, 은행 번호: 006, 지점: Hong Kong Branch, 지점 번호: 391, 주소: Citibank Tower, Citibank Plaza, 3 Garden Road, Central, Hong Kong)
> - 기타 통화 송금: **THE BANK OF NEW YORK MELLON**(SWIFT: `IRVTUS3NXXX`, 주소: 240 GREENWICH STREET, NEW YORK, United States)

## 라이선스

Apache-2.0
