<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# Ecat

[简体中文](../../../README.md) | [English](../../../README.en.md) | [日本語](../ja/README.md) | [한국어](../ko/README.md) | [Русский](../ru/README.md) | [Deutsch](../de/README.md) | [Français](../fr/README.md) | [Español](../es/README.md) | **[Português](../pt/README.md)** | [हिन्दी](../hi/README.md) | [العربية](../ar/README.md) | [বাংলা](../bn/README.md) | [Bahasa Indonesia](../id/README.md)

Nome chinês do Ecat: 一只猫

**一只猫** ("um gato") é um framework de microsserviços Rust comparável ao [go-kratos/kratos](https://github.com/go-kratos/kratos) v3 (v3.0.2 · 51 crates).

Oferece uma experiência de desenvolvimento API-first, arquitetura de componentes plugáveis, abstração unificada de middleware HTTP/gRPC e um conjunto completo de ferramentas CLI. Permite que desenvolvedores familiarizados com Kratos comecem sem atrito, aproveitando ao mesmo tempo a segurança de tipos, as abstrações de custo zero e o desempenho extremo do Rust.

<p align="center">
  <img src="e-cat.svg" alt="Mascote do projeto Ecat (dinâmico)" width="220" />
</p>

## Arquitetura de design

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

### Fluxo de processamento de requisições

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

## Recursos

- **API-first**: define APIs, códigos de erro e metadados com Protobuf; geração de código com prost + tonic-build
- **Suporte a dois protocolos**: HTTP (axum) e gRPC (tonic) compartilham o mesmo conjunto de middlewares `tower::Layer`
- **Arquitetura plugável**: Registry, Config, Logging e Encoding são todos abstraídos por traits, com implementações prontas para produção fornecidas por padrão
- **Sistema de middleware**: Recovery, Tracing, Logging, Timeout, RateLimit, Security, CircuitBreaker, MetricsLayer, RetryLayer, ValidateLayer, CORS (feature "cors") embutidos; combinados via `tower::ServiceBuilder`
- **Ciclo de vida da aplicação**: padrão Builder para construir o App, inicialização concorrente de múltiplos Servers, tratamento de sinais SIGTERM/SIGINT, hooks de ciclo de vida start/stop
- **Segurança de tipos**: sistema de códigos de erro baseado em protobuf, mapeamento de status HTTP em tempo de compilação
- **Observabilidade**: tracing + Prometheus + endpoints de Health (`/health`, `/ready`)
- **Detecção de ataques**: SecurityLayer detecta automaticamente SQL injection, XSS, SSRF e outros padrões de ataque, bloqueando requisições de alto risco
- **Comunicação entre serviços**: HttpClient integra descoberta de serviço e balanceamento de carga; CircuitBreaker protege contra falhas em cascata
- **Autenticação e autorização**: middlewares JWT / API Key, Claims propagados ao contexto da requisição
- **Mensagens e eventos**: trait MessageQueue + EventBus Pub/Sub local/remoto
- **Rastreamento distribuído**: spans de requisição, injeção/extração de trace_id
- **Cliente gRPC**: GrpcClient integra descoberta de serviço e balanceamento de carga
- **Múltiplos protocolos**: HTTP, gRPC, WebSocket e GraphQL roteados de forma unificada
- **Múltiplas fontes de dados**: RDBMS (SQLite/PG/MySQL/TiDB), cache (Redis/Memcached), busca (OpenSearch/Elasticsearch), grafos (Neo4j/NebulaGraph/ArangoDB), séries temporais (InfluxDB/IoTDB/QuestDB/TDengine), documentos (MongoDB), armazenamento de objetos (S3/MinIO)

### Mapeamento de conceitos do Kratos

| Kratos (Go) | e-cat (Rust) | Descrição |
|-------------|-------------|------|
| `kratos.New()` | `App::builder()` | Padrão Builder |
| `http.Handler` | `tower::Service` | Trait padrão do ecossistema Rust |
| `http.Server` | `axum::Router` | Framework HTTP mainstream da comunidade |
| `grpc.Server` | `tonic::transport::Server` | A implementação gRPC mais madura |
| `proto generate` | `prost + tonic-build` | Protobuf padrão da comunidade |
| `registry.Discovery` | `Registry` trait | Registro/descoberta plugável |
| `config.Source` | `ConfigSource` trait | Carregamento de configuração multi-fonte |

## Stack tecnológica

| Componente | Escolha |
|------|------|
| Runtime assíncrono | **tokio** |
| HTTP | **axum** |
| gRPC | **tonic** |
| Protobuf | **prost + tonic-build** |
| Middleware | **tower::Service / Layer** |
| Logs/rastreamento | **tracing + trace_id propagation** |
| Métricas | **prometheus** |
| Serialização | **serde + prost** |
| Detecção de ataques | **security-rust** |
| RDBMS | **sqlx** |
| Redis | **redis-rs** |
| JWT | **jsonwebtoken** |
| HTTP Client | **reqwest** |
| CLI | **clap** |

## Bancos de dados suportados

| Categoria | Banco de dados | Crate | Status |
|------|--------|-------|------|
| RDBMS | SQLite | `ecat-data-sqlx` | ✅ Implementado |
| RDBMS | PostgreSQL | `ecat-data-sqlx` | ✅ Implementado |
| RDBMS | MySQL | `ecat-data-sqlx` | ✅ Implementado |
| RDBMS | TiDB | `ecat-data-sqlx` | ✅ Implementado |
| Cache | Redis | `ecat-data-redis` | ✅ Implementado |
| Busca | OpenSearch | `ecat-data-opensearch` | ✅ Implementado |
| Busca | Elasticsearch | `ecat-data-elasticsearch` | ✅ Implementado |
| Cache | Memcached | `ecat-data-memcached` | ⚠️ Implementação em memória (não para produção, não use para cache persistente) |
| OLAP | ClickHouse | `ecat-data-clickhouse` | ✅ Implementado |
| Grafo | Neo4j | `ecat-data-neo4j` | ✅ API REST |
| Grafo | NebulaGraph | `ecat-data-nebulagraph` | ✅ API REST |
| Grafo | ArangoDB | `ecat-data-arangodb` | ✅ API REST |
| Séries temporais | InfluxDB | `ecat-data-influxdb` | ✅ API HTTP |
| Séries temporais | Apache IoTDB | `ecat-data-iotdb` | ✅ API REST |
| Séries temporais | QuestDB | `ecat-data-questdb` | ✅ API HTTP |
| Séries temporais | TDengine | `ecat-data-tdengine` | ✅ API REST |
| Documentos | MongoDB | `ecat-data-mongodb` | ✅ Driver nativo |
| Armazenamento de objetos | S3 / MinIO | `ecat-data-s3` | ✅ reqwest+rustls |

> Todos os backends de dados são abstraídos por traits unificados (`RdbmsClient` / `Cache` / `SearchClient` / `GraphClient` / `TsdbClient` / `DocumentClient` / `StorageClient`); importe o crate contrib correspondente conforme necessário. Cada backend fornece uma struct `XxxConfig` (`#[derive(Deserialize)]`) que suporta carregar as informações de conexão a partir de arquivos de configuração JSON/YAML.

> **Convenção de nomenclatura de construtores**: os crates de fila de mensagens (`ecat-mq-*`) usam `connect` como construtor principal (ex.: `KafkaMq::connect(brokers)`, `MqttMq::connect(url)`), além de `from_config` para carregar da configuração; a maioria dos crates de backend de dados (`ecat-data-*`) usa `new` como construtor principal, com exceções: `ecat-data-redis` / `ecat-data-sqlx` mantêm `connect`, e `ecat-data-mongodb` / `ecat-data-s3` oferecem apenas `from_config`. Trata-se de uma convenção existente, não unificada à força (para evitar mudanças que quebrem código); pode ser avaliada uma unificação na janela 3.0.

### Exemplo de configuração de banco de dados

Cada backend de dados fornece uma struct `XxxConfig` e o método `from_config()`, que desacoplam as informações de conexão do código para arquivos de configuração:

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

**Referência de campos de configuração**:

| Backend | Config | Campos | Valor de exemplo |
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

> Todos os Configs de backend suportam o campo opcional `tls` (`TlsClientConfig`), usado para configurar a autenticação por certificado de cliente TLS. Consulte o [Tutorial de configuração de banco de dados](database-config-tutorial.md).

## Estrutura do projeto

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

## Início rápido

### Pré-requisitos

- Rust 1.85+ (toolchain stable, exigência da edition 2024)
- [protoc](https://github.com/protocolbuffers/protobuf) (compilador Protocol Buffers)

### Instalar o CLI

```bash
cargo install ecat-cli
```

### Criar um serviço

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

Acesse `http://localhost:8000/helloworld/ecat`.

### Exemplo de código

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

`ecat` fornece pontos de re-export controlados por features — habilite apenas os componentes necessários:

```rust
use ecat::transport_http::HttpServer;   // feature "http"（默认）
use ecat::middleware::RecoveryLayer;     // feature "middleware"
use ecat::auth::JwtAuthLayer;            // feature "auth"
use ecat::data::redis::RedisCache;       // feature "redis"
```

Features padrão = `http+grpc`; use `--no-default-features --features <componente>` para enxugar a árvore de dependências. Lista completa de features: `http` `grpc` `middleware` `auth` `client` `events` `metrics` `tracing` `circuit-breaker` `consul` `remote` `redis`.

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

> Observação: `ecat_middleware::TracingLayer` não injeta trace_id; para injeção de trace_id em nível de requisição, use `ecat_tracing::TracingLayer::new()`.

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

### Tratamento de erros

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

## Fases de implementação

| Fase | Status | Conteúdo |
|------|------|------|
| Phase 1 | ✅ Concluída | Esqueleto do projeto, protos, errors, metadata, encoding, logging |
| Phase 2 | ✅ Concluída | Camada Transport (HTTP + gRPC) |
| Phase 3 | ✅ Concluída | Sistema de Middleware (Recovery/Tracing/Logging/Timeout) |
| Phase 4 | ✅ Concluída | Gerenciamento do ciclo de vida do App |
| Phase 5 | ✅ Concluída | Registry, Config, Metrics |
| Phase 5.5 | ✅ Concluída | Camada de acesso a dados (traits + backend sqlx) |
| Phase 6 | ✅ Concluída | Ferramentas CLI (new/proto/run/build) |
| Phase 7 | ✅ Concluída | README, exemplos (helloworld), documentos de design |
| Phase 8 | ✅ Concluída | Integração de detecção de ataques (security-rust, ecat-security) |
| Phase 9 | ✅ Concluída | Ecossistema fase 1 (health / client / circuit-breaker / auth / registry-consul) |
| Phase 10 | ✅ Concluída | Ecossistema fase 2 (redis / mq / events / config-remote) |
| Phase 11 | ✅ Concluída | Ecossistema fase 3 (testing / deploy / bench / openapi) |
| Phase 12 | ✅ Concluída | Comunicação e reforço de segurança (cliente gRPC / OAuth2 / mTLS / rastreamento distribuído) |
| Phase 13 | ✅ Concluída | Complemento de backends de dados (etcd / Kafka / OpenSearch / InfluxDB) |
| Phase 14 | ✅ Concluída | Operações e experiência (WebSocket / versionamento de API / Helm / CI/CD) |
| Phase 15 | ✅ Concluída | Expansão do ecossistema v2 (Kafka real / RabbitMQ / MQTT / NATS / MongoDB / S3 / TDengine / OTLP / lock distribuído / scheduler / CLI watch+upgrade) |
| Phase 16 | ✅ Concluída | Manutenção reforçada v2.4 (M1 MetricsLayer / M2 RetryLayer / M3 ValidateLayer / M4 CORS / U1 crate agregado ecat / U2 examples / hash de token OAuth2 / rastreamento de CVE) |

## Limitações conhecidas

- **Parsing GraphQL (ecat-graphql)**: suporta argumentos de campo e selections aninhadas (`query_field`/`mutation_field` resolvers ricos podem acessar `args`/`variables`/`selection`); ainda não suporta aliases, fragments nem múltiplos campos de nível superior — não o exponha como endpoint GraphQL genérico.
- **Cache de introspecção OAuth2 (ecat-auth)**: a chave do cache é o hash SHA-256 do token (o token em texto puro não é armazenado); o valor em cache é filtrado por whitelist (por padrão mantém sub/exp/iat/role + iss/aud/scope/roles do extra, configurável via `cache_claims_whitelist`; em caso de miss, as claims completas ainda são retornadas — apenas o valor em cache é filtrado); entradas expiradas pelo TTL são removidas ativamente na escrita (TTL padrão 300s).
- **Kafka offset (ecat-mq-kafka)**: por padrão `enable.auto.commit=false` e sem commit manual — após reinício do processo, a leitura recomeça do fim da partição (latest), e mensagens produzidas durante a parada são puladas; é necessário configurar explicitamente `auto_commit=true` para obter semântica at-least-once (após reinício, continua do último ponto de commit).

## Objetivos de design

| # | Objetivo | Descrição |
|---|------|------|
| 1 | **Alinhamento com Kratos** | Manter a filosofia API-first, plugável e de abstração unificada do Kratos |
| 2 | **Idiomático em Rust** | Reutilizar tower::Service, generics de trait, abstrações de custo zero; não fazer "Go in Rust" |
| 3 | **Segurança de tipos** | Capturar erros em tempo de compilação; definições Protobuf totalmente tipadas |
| 4 | **Plugável** | Registry, Config, Logging, Encoding todos abstraídos por traits |
| 5 | **Toolchain completo** | CLI suporta scaffolding de projetos, geração de código proto, execução em desenvolvimento |
| 6 | **Performance em primeiro lugar** | Abstrações de custo zero + runtime assíncrono |
| 7 | **Observável** | tracing + Prometheus prontos para uso |
| 8 | **Ecossistema completo** | Cliente, circuit breaker, autenticação, health check, backends de registry |

## Notas técnicas

### Por que tower::Service

[`tower::Service`](https://docs.rs/tower/latest/tower/trait.Service.html) é o equivalente ao `http.Handler` do ecossistema assíncrono Rust. Tanto axum quanto tonic são construídos sobre tower; portanto, o e-cat não precisa de um trait de middleware próprio — basta fornecer implementações de `tower::Layer` para alcançar o mesmo efeito dos middlewares do Kratos, sem custo de adaptadores.

### Por que Cargo Workspace

Consistente com o design modular do Kratos. Todos os crates `ecat-*` são publicados em versões sincronizadas no workspace (atualmente 3.0.2), compilados de forma independente; o usuário importa conforme necessário. Os crates centrais mantêm dependências mínimas; os crates contrib fornecem integrações opcionais.

### Por que prost (e não protobuf-rs)

prost é a implementação de protobuf mais amplamente usada na comunidade Rust; gera código seguro por tipos em tempo de compilação e integra-se profundamente com tonic.

## Documentos de design

- [设计规范](../../../docs/superpowers/specs/2026-07-29-ecat-framework-design.md)
- [实现计划](../../../docs/superpowers/plans/2026-07-29-ecat-framework.md)
- [Plano de ecossistema v1](ecosystem-plan.md) (concluído)
- [Plano de ecossistema v2](ecosystem-plan-v2.md) (concluído)
- [Plano de ecossistema v3](ecosystem-plan-v3.md) (avaliação final)
- [Referência da API](api.md)
- [Relatório de auditoria r5](audit-report-2026-08-01-r5.md) (2026-08-01)
- [Tutorial de configuração de banco de dados](database-config-tutorial.md)
- [Rastreamento de CVE de dependências](dependency-cve-tracking.md)
- [Tutorial de certificados TLS](tls-certificate-tutorial.md)
- [Arquivo de configuração de exemplo](../../../config/databases.example.yaml)

## Suporte

Apoie este projeto!

| WeChat Pay | Alipay |
|:---:|:---:|
| <img src="weixinpay.png" width="130" height="130" alt="WeChat Pay"> | <img src="alipay.png" width="130" height="130" alt="Alipay"> |

### Transferência bancária global

| Item | Informação |
|------|------|
| Nome do beneficiário | WANG KEXUN |
| Número da conta do beneficiário | 881015918251 |
| Banco do beneficiário | ZA Bank Limited |
| SWIFT Code | AABLHKHHXXX |
| Código do banco | 387 |
| Endereço do banco | Core F, Cyberport 3, 100 Cyberport Road, Hong Kong |

> **Banco correspondente para transferências internacionais (se necessário)**: estas são as informações do banco correspondente (banco intermediário), não do banco beneficiário. Consulte o seu banco remetente para saber se é necessário fornecê-las.
>
> - Para remessas em dólares de Hong Kong (HKD), renminbi (RMB) e dólares americanos (USD): **Citibank N.A. Hong Kong** (SWIFT: `CITIHKHXXXX`, código do banco: 006, filial: Hong Kong Branch, código da filial: 391, endereço: Citibank Tower, Citibank Plaza, 3 Garden Road, Central, Hong Kong)
> - Para remessas em outras moedas: **THE BANK OF NEW YORK MELLON** (SWIFT: `IRVTUS3NXXX`, endereço: 240 GREENWICH STREET, NEW YORK, United States)

## Licença

Apache-2.0
