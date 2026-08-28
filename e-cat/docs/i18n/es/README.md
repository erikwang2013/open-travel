<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# Ecat

[简体中文](../../../README.md) | [English](../../../README.en.md) | [日本語](../ja/README.md) | [한국어](../ko/README.md) | [Русский](../ru/README.md) | [Deutsch](../de/README.md) | [Français](../fr/README.md) | **[Español](../es/README.md)** | [Português](../pt/README.md) | [हिन्दी](../hi/README.md) | [العربية](../ar/README.md) | [বাংলা](../bn/README.md) | [Bahasa Indonesia](../id/README.md)

Nombre chino de Ecat: 一只猫 (literalmente "un gato")

**一只猫** es un framework de microservicios en Rust inspirado en [go-kratos/kratos](https://github.com/go-kratos/kratos) v3 (v3.0.2 · 51 crates).

Ofrece una experiencia de desarrollo API-first, una arquitectura de componentes enchufables, una abstracción unificada de middleware HTTP/gRPC y una cadena de herramientas CLI completa. Los desarrolladores familiarizados con Kratos pueden empezar sin fricciones, aprovechando al mismo tiempo la seguridad de tipos, las abstracciones de costo cero y el rendimiento extremo de Rust.

<p align="center">
  <img src="e-cat.svg" alt="Mascota del proyecto Ecat (animada)" width="220" />
</p>

## Arquitectura de diseño

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

### Flujo de procesamiento de peticiones

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

## Características

- **API-first**: define API, códigos de error y metadatos con Protobuf; generación de código con prost + tonic-build
- **Soporte de doble protocolo**: HTTP (axum) y gRPC (tonic) comparten el mismo conjunto de middleware tower::Layer
- **Arquitectura enchufable**: Registry, Config, Logging y Encoding están abstraídos mediante traits, con implementaciones listas para producción por defecto
- **Sistema de middleware**: incluye Recovery, Tracing, Logging, Timeout, RateLimit, Security, CircuitBreaker, MetricsLayer, RetryLayer, ValidateLayer y CORS (feature "cors"); se compone mediante tower::ServiceBuilder
- **Ciclo de vida de la aplicación**: construcción de App con patrón Builder, arranque concurrente de múltiples servidores, manejo de señales SIGTERM/SIGINT y hooks de ciclo de vida start/stop
- **Seguridad de tipos**: sistema de códigos de error basado en protobuf, mapeo de códigos de estado HTTP en tiempo de compilación
- **Observabilidad**: tracing + Prometheus + endpoints de salud (/health, /ready)
- **Detección de ataques**: SecurityLayer detecta automáticamente patrones de ataque como inyección SQL, XSS y SSRF, bloqueando las peticiones de alto riesgo
- **Comunicación entre servicios**: HttpClient integra descubrimiento de servicios y balanceo de carga; CircuitBreaker proporciona protección por disyuntor
- **Autenticación y autorización**: middleware de autenticación JWT / API Key, con Claims propagados al contexto de la petición
- **Mensajes y eventos**: trait MessageQueue + EventBus para Pub/Sub local y remoto
- **Trazado distribuido**: spans de petición, inyección/extracción de trace_id
- **Cliente gRPC**: GrpcClient integra descubrimiento de servicios y balanceo de carga
- **Multiprotocolo**: enrutamiento unificado de HTTP, gRPC, WebSocket y GraphQL
- **Múltiples fuentes de datos**: RDBMS (SQLite/PG/MySQL/TiDB), caché (Redis/Memcached), búsqueda (OpenSearch/Elasticsearch), grafos (Neo4j/NebulaGraph/ArangoDB), series temporales (InfluxDB/IoTDB/QuestDB/TDengine), documentos (MongoDB), almacenamiento de objetos (S3/MinIO)

### Mapeo de conceptos de Kratos

| Kratos (Go) | e-cat (Rust) | Descripción |
|-------------|-------------|------|
| `kratos.New()` | `App::builder()` | Patrón Builder |
| `http.Handler` | `tower::Service` | Trait estándar del ecosistema Rust |
| `http.Server` | `axum::Router` | Framework HTTP más usado por la comunidad |
| `grpc.Server` | `tonic::transport::Server` | La implementación de gRPC más madura |
| `proto generate` | `prost + tonic-build` | Protobuf estándar de la comunidad |
| `registry.Discovery` | `Registry` trait | Registro y descubrimiento enchufables |
| `config.Source` | `ConfigSource` trait | Carga de configuración de múltiples fuentes |

## Stack tecnológico

| Componente | Elección |
|------|------|
| Runtime asíncrono | **tokio** |
| HTTP | **axum** |
| gRPC | **tonic** |
| Protobuf | **prost + tonic-build** |
| Middleware | **tower::Service / Layer** |
| Logs/trazado | **tracing + propagación de trace_id** |
| Métricas | **prometheus** |
| Serialización | **serde + prost** |
| Detección de ataques | **security-rust** |
| RDBMS | **sqlx** |
| Redis | **redis-rs** |
| JWT | **jsonwebtoken** |
| Cliente HTTP | **reqwest** |
| CLI | **clap** |

## Bases de datos compatibles

| Categoría | Base de datos | Crate | Estado |
|------|--------|-------|------|
| RDBMS | SQLite | `ecat-data-sqlx` | ✅ Implementado |
| RDBMS | PostgreSQL | `ecat-data-sqlx` | ✅ Implementado |
| RDBMS | MySQL | `ecat-data-sqlx` | ✅ Implementado |
| RDBMS | TiDB | `ecat-data-sqlx` | ✅ Implementado |
| Caché | Redis | `ecat-data-redis` | ✅ Implementado |
| Búsqueda | OpenSearch | `ecat-data-opensearch` | ✅ Implementado |
| Búsqueda | Elasticsearch | `ecat-data-elasticsearch` | ✅ Implementado |
| Caché | Memcached | `ecat-data-memcached` | ⚠️ Implementación en memoria (no apta para producción; no usar como caché persistente) |
| OLAP | ClickHouse | `ecat-data-clickhouse` | ✅ Implementado |
| Grafos | Neo4j | `ecat-data-neo4j` | ✅ API REST |
| Grafos | NebulaGraph | `ecat-data-nebulagraph` | ✅ API REST |
| Grafos | ArangoDB | `ecat-data-arangodb` | ✅ API REST |
| Series temporales | InfluxDB | `ecat-data-influxdb` | ✅ API HTTP |
| Series temporales | Apache IoTDB | `ecat-data-iotdb` | ✅ API REST |
| Series temporales | QuestDB | `ecat-data-questdb` | ✅ API HTTP |
| Series temporales | TDengine | `ecat-data-tdengine` | ✅ API REST |
| Documentos | MongoDB | `ecat-data-mongodb` | ✅ Driver nativo |
| Almacenamiento de objetos | S3 / MinIO | `ecat-data-s3` | ✅ reqwest+rustls |

> Todos los backends de datos se abstraen mediante un trait unificado (`RdbmsClient` / `Cache` / `SearchClient` / `GraphClient` / `TsdbClient` / `DocumentClient` / `StorageClient`); se incorpora el crate contrib correspondiente según necesidad. Cada backend ofrece una estructura `XxxConfig` (`#[derive(Deserialize)]`) que admite cargar la información de conexión desde archivos de configuración JSON/YAML.

> **Convención de nombres de constructores**: los crates de colas de mensajes (`ecat-mq-*`) usan `connect` como constructor principal (p. ej. `KafkaMq::connect(brokers)`, `MqttMq::connect(url)`), además de `from_config` para cargar desde configuración; en los crates de backends de datos (`ecat-data-*`), el constructor principal de la mayoría es `new`, con excepciones: `ecat-data-redis` / `ecat-data-sqlx` mantienen `connect`, y `ecat-data-mongodb` / `ecat-data-s3` solo ofrecen `from_config`. Esta es una convención existente que no se fuerza a unificar (para evitar cambios disruptivos); la ventana 3.0 puede evaluar la unificación.

### Ejemplo de configuración de base de datos

Cada backend de datos ofrece la estructura `XxxConfig` y el método `from_config()` para desacoplar la información de conexión del código y llevarla al archivo de configuración:

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

**Referencia de campos de configuración**:

| Backend | Config | Campos | Ejemplo de valor |
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
| Memcached | `MemcachedConfig` | `username`?, `password`? (campos reservados) | — |
| TDengine | `TdengineConfig` | `base_url`, `username`, `password`, `database`? | `http://localhost:6041` |
| MongoDB | `MongoConfig` | `url`, `database`, `tls`? | `mongodb://localhost:27017`, `app` |
| S3 | `S3Config` | `endpoint`, `region`, `access_key`, `secret_key`, `tls`? | `http://localhost:9000`, `us-east-1` |

> Todos los backends Config admiten un campo opcional `tls` (`TlsClientConfig`) para configurar la autenticación con certificados de cliente TLS. Consulta el [Tutorial de configuración de base de datos](database-config-tutorial.md).

## Estructura del proyecto

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

## Inicio rápido

### Requisitos previos

- Rust 1.85+ (cadena de herramientas stable, requisito de edition 2024)
- [protoc](https://github.com/protocolbuffers/protobuf) (compilador de Protocol Buffers)

### Instalar la CLI

```bash
cargo install ecat-cli
```

### Crear un servicio

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

Visita `http://localhost:8000/helloworld/ecat`.

### Ejemplo de código

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

### Crate agregado (ecat)

`ecat` ofrece un punto de entrada de re-export controlado por features: habilita solo los componentes que necesites:

```rust
use ecat::transport_http::HttpServer;   // feature "http"（默认）
use ecat::middleware::RecoveryLayer;     // feature "middleware"
use ecat::auth::JwtAuthLayer;            // feature "auth"
use ecat::data::redis::RedisCache;       // feature "redis"
```

Features por defecto = `http+grpc`; usa `--no-default-features --features <componente>` para reducir el árbol de dependencias. Lista completa de features: `http` `grpc` `middleware` `auth` `client` `events` `metrics` `tracing` `circuit-breaker` `consul` `remote` `redis`.

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

> Nota: `ecat_middleware::TracingLayer` no inyecta trace_id; si necesitas inyección de trace_id a nivel de petición, usa `ecat_tracing::TracingLayer::new()`.

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

### Manejo de errores

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

## Fases de implementación

| Fase | Estado | Contenido |
|------|------|------|
| Phase 1 | ✅ Completada | Esqueleto del proyecto, protos, errors, metadata, encoding, logging |
| Phase 2 | ✅ Completada | Capa Transport (HTTP + gRPC) |
| Phase 3 | ✅ Completada | Sistema de middleware (Recovery/Tracing/Logging/Timeout) |
| Phase 4 | ✅ Completada | Gestión del ciclo de vida de la App |
| Phase 5 | ✅ Completada | Registry, Config, Metrics |
| Phase 5.5 | ✅ Completada | Capa de acceso a datos (traits + backend sqlx) |
| Phase 6 | ✅ Completada | Cadena de herramientas CLI (new/proto/run/build) |
| Phase 7 | ✅ Completada | README, ejemplos (helloworld), documentos de diseño |
| Phase 8 | ✅ Completada | Integración de detección de ataques (security-rust, ecat-security) |
| Phase 9 | ✅ Completada | Primera fase del ecosistema (health / client / circuit-breaker / auth / registry-consul) |
| Phase 10 | ✅ Completada | Segunda fase del ecosistema (redis / mq / events / config-remote) |
| Phase 11 | ✅ Completada | Tercera fase del ecosistema (testing / deploy / bench / openapi) |
| Phase 12 | ✅ Completada | Refuerzo de comunicaciones y seguridad (cliente gRPC / OAuth2 / mTLS / trazado distribuido) |
| Phase 13 | ✅ Completada | Completado de backends de datos (etcd / Kafka / OpenSearch / InfluxDB) |
| Phase 14 | ✅ Completada | Operaciones y experiencia (WebSocket / gestión de versiones de API / Helm / CI/CD) |
| Phase 15 | ✅ Completada | Extensión del ecosistema v2 (Kafka real / RabbitMQ / MQTT / NATS / MongoDB / S3 / TDengine / OTLP / bloqueo distribuido / scheduler / CLI watch+upgrade) |
| Phase 16 | ✅ Completada | Refuerzo de mantenimiento v2.4 (M1 MetricsLayer / M2 RetryLayer / M3 ValidateLayer / M4 CORS / U1 crate agregado ecat / U2 examples / hash de token OAuth2 / seguimiento CVE) |

## Limitaciones conocidas

- **Análisis GraphQL (ecat-graphql)**: admite parámetros de campo y selections anidadas (los resolvers enriquecidos `query_field`/`mutation_field` pueden acceder a `args`/`variables`/`selection`); aún no admite alias, fragments ni múltiples campos de nivel superior; no lo expongas como endpoint GraphQL genérico.
- **Caché de introspección OAuth2 (ecat-auth)**: la clave de caché es el hash SHA-256 del token (no se almacena el token en claro); el valor en caché se filtra con una lista blanca (por defecto conserva sub/exp/iat/role + iss/aud/scope/roles de extra; `cache_claims_whitelist` es configurable; en caso de miss se sigue devolviendo el conjunto completo de claims, solo se filtra el valor en caché); las entradas expiradas por TTL se purgan activamente al escribir (TTL por defecto 300 s).
- **Offsets de Kafka (ecat-mq-kafka)**: por defecto `enable.auto.commit=false` y sin commit manual: tras un reinicio del proceso se vuelve a leer desde el final de la partición (latest), y los mensajes producidos durante la parada se omiten; solo con `auto_commit=true` se obtiene semántica at-least-once (el reinicio continúa desde el último punto confirmado).

## Objetivos de diseño

| # | Objetivo | Descripción |
|---|------|------|
| 1 | **Alineación con Kratos** | Mantener la filosofía API-first, enchufable y de abstracción unificada de Kratos |
| 2 | **Idiomático en Rust** | Reutilizar tower::Service, traits genéricos y abstracciones de costo cero; no hacer "Go in Rust" |
| 3 | **Seguridad de tipos** | Capturar errores en tiempo de compilación; definiciones Protobuf totalmente tipadas |
| 4 | **Enchufable** | Registry, Config, Logging y Encoding abstraídos mediante traits |
| 5 | **Cadena de herramientas completa** | CLI con scaffolding de proyectos, generación de código proto y ejecución en desarrollo |
| 6 | **Rendimiento primero** | Abstracciones de costo cero + runtime asíncrono |
| 7 | **Observable** | tracing + Prometheus listos para usar |
| 8 | **Ecosistema completo** | Clientes, disyuntor, autenticación, salud, backends de registro |

## Notas técnicas

### Por qué tower::Service

[`tower::Service`](https://docs.rs/tower/latest/tower/trait.Service.html) es el equivalente de `http.Handler` en el ecosistema asíncrono de Rust. Tanto axum como tonic se construyen sobre tower, por lo que e-cat no necesita un trait de middleware propio: basta con proporcionar implementaciones de tower::Layer para lograr el mismo efecto que los middleware de Kratos, sin coste de adaptadores.

### Por qué un Cargo Workspace

De acuerdo con el diseño modular de Kratos. Todos los crates `ecat-*` se publican en versiones sincronizadas del workspace (actualmente 3.0.2), se compilan de forma independiente y el usuario los incorpora según necesidad. Los crates núcleo mantienen dependencias mínimas; los crates contrib ofrecen integraciones opcionales.

### Por qué prost (en lugar de protobuf-rs)

prost es la implementación de protobuf más utilizada en la comunidad Rust; genera código seguro por tipos en tiempo de compilación y está profundamente integrado con tonic.

## Documentos de diseño

- [Especificación de diseño](../../../docs/superpowers/specs/2026-07-29-ecat-framework-design.md)
- [Plan de implementación](../../../docs/superpowers/plans/2026-07-29-ecat-framework.md)
- [Plan del ecosistema v1](ecosystem-plan.md) (completado)
- [Plan del ecosistema v2](ecosystem-plan-v2.md) (completado)
- [Plan del ecosistema v3](ecosystem-plan-v3.md) (evaluación final)
- [Referencia de API](api.md)
- [Informe de auditoría r5](audit-report-2026-08-01-r5.md) (2026-08-01)
- [Tutorial de configuración de base de datos](database-config-tutorial.md)
- [Seguimiento de CVE de dependencias](dependency-cve-tracking.md)
- [Tutorial de autenticación con certificados TLS](tls-certificate-tutorial.md)
- [Archivo de ejemplo de configuración](../../../config/databases.example.yaml)

## Soporte

¡Gracias por apoyar este proyecto!

| WeChat Pay | Alipay |
|:---:|:---:|
| <img src="weixinpay.png" width="130" height="130" alt="WeChat Pay"> | <img src="alipay.png" width="130" height="130" alt="Alipay"> |

### Transferencia global (transferencia bancaria)

| Campo | Información |
|------|------|
| Nombre del beneficiario | WANG KEXUN |
| Número de cuenta del beneficiario | 881015918251 |
| Banco beneficiario | ZA Bank Limited |
| SWIFT Code | AABLHKHHXXX |
| Código de banco | 387 |
| Dirección del banco | Core F, Cyberport 3, 100 Cyberport Road, Hong Kong |

> **Banco corresponsal para transferencias transfronterizas (si es necesario)**: esta es la información del banco corresponsal (banco intermediario), no del banco beneficiario; consulta con tu banco emisor si es necesario proporcionarla.
>
> - Transferencias en HKD, CNY y USD: **Citibank N.A. Hong Kong** (SWIFT: `CITIHKHXXXX`, código de banco: 006, sucursal: Hong Kong Branch, código de sucursal: 391, dirección: Citibank Tower, Citibank Plaza, 3 Garden Road, Central, Hong Kong)
> - Transferencias en otras divisas: **THE BANK OF NEW YORK MELLON** (SWIFT: `IRVTUS3NXXX`, dirección: 240 GREENWICH STREET, NEW YORK, United States)

## Licencia

Apache-2.0
