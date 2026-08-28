<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# Ecat

[简体中文](../../../README.md) | [English](../../../README.en.md) | [日本語](../ja/README.md) | [한국어](../ko/README.md) | [Русский](../ru/README.md) | **[Deutsch](../de/README.md)** | [Français](../fr/README.md) | [Español](../es/README.md) | [Português](../pt/README.md) | [हिन्दी](../hi/README.md) | [العربية](../ar/README.md) | [বাংলা](../bn/README.md) | [Bahasa Indonesia](../id/README.md)

Der chinesische Name von Ecat: 一只猫 (wörtlich „eine Katze")

**一只猫** ist ein Rust-Mikroservice-Framework, das sich an [go-kratos/kratos](https://github.com/go-kratos/kratos) v3 orientiert (v3.0.2 · 51 crates).

Es bietet eine API-first-Entwicklungserfahrung, eine pluggbare Komponentenarchitektur, eine einheitliche HTTP/gRPC-Middleware-Abstraktion sowie eine vollständige CLI-Werkzeugkette. Entwickler, die Kratos kennen, können nahtlos einsteigen und gleichzeitig die Typsicherheit, Zero-Cost-Abstraktionen und die extreme Leistung von Rust voll ausschöpfen.

<p align="center">
  <img src="e-cat.svg" alt="Ecat-Projektmaskottchen (animiert)" width="220" />
</p>

## Designarchitektur

```
┌──────────────────────────────────────────────────────────────┐
│                         ecat-cli                             │
│        (new │ proto │ run --watch │ build │ upgrade)         │
├──────────────────────────────────────────────────────────────┤
│                     ecat (Anwendungslebenszyklus)            │
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
│                         Daten-Ebene                          │
│     ────────────────────────────────────────────────          │
│     rdbms:   SQLite / PostgreSQL / MySQL / TiDB              │
│     cache:   Redis ✓                                         │
│     config:  remote (Consul KV)                              │
│     registry: consul                                         │
├──────────────────────────────────────────────────────────────┤
│                       ecat-protos                             │
│     (gemeinsame .proto-Definitionen: errors, metadata, ...)  │
└──────────────────────────────────────────────────────────────┘
```

### Anfrageverarbeitungsablauf

```
Client-Anfrage
  │
  ├─ HTTP 0.0.0.0:8000 ──→ axum::Router ──┐
  │                                        │
  └─ gRPC 0.0.0.0:9000 ──→ tonic::Server ─┤
                                      │
                              ┌───────┴───────┐
                              │   Middleware   │
                              │   ──────────   │
                              │ 1. Recovery    │  fängt Panics ab
                              │ 2. Tracing     │  injiziert trace_id
                              │ 3. Logging     │  Anfrageprotokoll
                              │ 4. Auth        │  Authentifizierung/Autorisierung
                              │ 5. Metrics     │  Metrikerfassung
│ 6. Security    │  Angriffserkennung
│ 7. CircuitBrk  │  Schutz durch Leistungsschalter
                              └───────┬───────┘
                                      │
                              ┌───────┴───────┐
                              │    Handler     │  Benutzer-Geschäftslogik
                              │ (tower::Service)│
                              └───────┬───────┘
                                      │
                              ┌───────┴───────┐
                              │   Response     │  Codierung/Serialisierung
                              │ JSON/Protobuf  │
                              └───────────────┘
```

## Funktionen

- **API-first**: APIs, Fehlercodes und Metadaten per Protobuf definieren; Codegenerierung mit prost + tonic-build
- **Zwei Protokolle**: HTTP (axum) und gRPC (tonic) teilen sich dieselbe tower::Layer-Middleware
- **Pluggable Architektur**: Registry, Config, Logging, Encoding — alles über Traits abstrahiert, produktionsreife Standardimplementierungen enthalten
- **Middleware-System**: eingebaut: Recovery, Tracing, Logging, Timeout, RateLimit, Security, CircuitBreaker, MetricsLayer, RetryLayer, ValidateLayer, CORS (cors-Feature); kombinierbar über tower::ServiceBuilder
- **Anwendungslebenszyklus**: App im Builder-Muster, paralleler Start mehrerer Server, SIGTERM/SIGINT-Signalbehandlung, start/stop-Lebenszyklus-Hooks
- **Typsicherheit**: protobuf-basiertes Fehlercodesystem mit HTTP-Statuscode-Zuordnung zur Compilezeit
- **Beobachtbarkeit**: tracing + Prometheus + Health-Endpunkte (/health, /ready)
- **Angriffserkennung**: SecurityLayer erkennt automatisch SQL-Injection, XSS, SSRF u. a. Angriffsmuster und blockiert kritische Anfragen
- **Dienstkommunikation**: HttpClient integriert Service Discovery und Load Balancing; CircuitBreaker-Schutz
- **Authentifizierung/Autorisierung**: JWT-/API-Key-Authentifizierungs-Middleware, Claims werden in den Request-Kontext übergeben
- **Nachrichten und Ereignisse**: MessageQueue-Trait + EventBus für lokales/entferntes Pub/Sub
- **Verteilte Ablaufverfolgung**: Request-Spans, trace_id-Injektion/-Extraktion
- **gRPC-Client**: GrpcClient integriert Service Discovery und Load Balancing
- **Mehrere Protokolle**: einheitliches Routing für HTTP, gRPC, WebSocket und GraphQL
- **Mehrere Datenquellen**: RDBMS (SQLite/PG/MySQL/TiDB), Cache (Redis/Memcached), Suche (OpenSearch/Elasticsearch), Graph (Neo4j/NebulaGraph/ArangoDB), Zeitreihen (InfluxDB/IoTDB/QuestDB/TDengine), Dokumente (MongoDB), Objektspeicher (S3/MinIO)

### Kratos-Konzept-Zuordnung

| Kratos (Go) | e-cat (Rust) | Beschreibung |
|-------------|-------------|------|
| `kratos.New()` | `App::builder()` | Builder-Muster |
| `http.Handler` | `tower::Service` | Standard-Trait der Rust-Ökosystems |
| `http.Server` | `axum::Router` | Gängiges Community-HTTP-Framework |
| `grpc.Server` | `tonic::transport::Server` | Ausgereifteste gRPC-Implementierung |
| `proto generate` | `prost + tonic-build` | Community-Standard-Protobuf |
| `registry.Discovery` | `Registry`-Trait | Pluggable Registrierung/Discovery |
| `config.Source` | `ConfigSource`-Trait | Konfigurationsladung aus mehreren Quellen |

## Technologie-Stack

| Komponente | Auswahl |
|------|------|
| Async-Runtime | **tokio** |
| HTTP | **axum** |
| gRPC | **tonic** |
| Protobuf | **prost + tonic-build** |
| Middleware | **tower::Service / Layer** |
| Logging/Tracing | **tracing + trace_id-Propagation** |
| Metriken | **prometheus** |
| Serialisierung | **serde + prost** |
| Angriffserkennung | **security-rust** |
| RDBMS | **sqlx** |
| Redis | **redis-rs** |
| JWT | **jsonwebtoken** |
| HTTP-Client | **reqwest** |
| CLI | **clap** |

## Unterstützte Datenbanken

| Kategorie | Datenbank | Crate | Status |
|------|--------|-------|------|
| RDBMS | SQLite | `ecat-data-sqlx` | ✅ Implementiert |
| RDBMS | PostgreSQL | `ecat-data-sqlx` | ✅ Implementiert |
| RDBMS | MySQL | `ecat-data-sqlx` | ✅ Implementiert |
| RDBMS | TiDB | `ecat-data-sqlx` | ✅ Implementiert |
| Cache | Redis | `ecat-data-redis` | ✅ Implementiert |
| Suche | OpenSearch | `ecat-data-opensearch` | ✅ Implementiert |
| Suche | Elasticsearch | `ecat-data-elasticsearch` | ✅ Implementiert |
| Cache | Memcached | `ecat-data-memcached` | ⚠️ Speicherimplementierung (nicht produktionsreif, nicht für persistenten Cache verwenden) |
| OLAP | ClickHouse | `ecat-data-clickhouse` | ✅ Implementiert |
| Graph | Neo4j | `ecat-data-neo4j` | ✅ REST-API |
| Graph | NebulaGraph | `ecat-data-nebulagraph` | ✅ REST-API |
| Graph | ArangoDB | `ecat-data-arangodb` | ✅ REST-API |
| Zeitreihen | InfluxDB | `ecat-data-influxdb` | ✅ HTTP-API |
| Zeitreihen | Apache IoTDB | `ecat-data-iotdb` | ✅ REST-API |
| Zeitreihen | QuestDB | `ecat-data-questdb` | ✅ HTTP-API |
| Zeitreihen | TDengine | `ecat-data-tdengine` | ✅ REST-API |
| Dokumente | MongoDB | `ecat-data-mongodb` | ✅ Nativ-Treiber |
| Objektspeicher | S3 / MinIO | `ecat-data-s3` | ✅ reqwest+rustls |

> Alle Daten-Backends sind über einheitliche Traits abstrahiert (`RdbmsClient` / `Cache` / `SearchClient` / `GraphClient` / `TsdbClient` / `DocumentClient` / `StorageClient`); die jeweiligen Contrib-Crates werden nach Bedarf eingebunden. Jedes Backend stellt eine `XxxConfig`-Struktur (`#[derive(Deserialize)]`) bereit, die das Laden der Verbindungsinformationen aus JSON-/YAML-Konfigurationsdateien unterstützt.

> **Namenskonvention für Konstruktoren**: Der Hauptkonstruktor der Message-Queue-Crates (`ecat-mq-*`) ist einheitlich `connect` (z. B. `KafkaMq::connect(brokers)`, `MqttMq::connect(url)`), zusätzlich gibt es `from_config` zum Laden aus der Konfiguration; bei den Daten-Backend-Crates (`ecat-data-*`) ist der Hauptkonstruktor meist `new`, Ausnahmen: `ecat-data-redis` / `ecat-data-sqlx` verwenden weiterhin `connect`, `ecat-data-mongodb` / `ecat-data-s3` bieten nur `from_config` an. Dies ist eine bestehende Konvention und wird nicht zwangsvereinheitlicht (um Breaking Changes zu vermeiden); eine Vereinheitlichung kann im 3.0-Fenster geprüft werden.

### Beispiel für die Datenbankkonfiguration

Jedes Daten-Backend bietet eine `XxxConfig`-Struktur und eine `from_config()`-Methode, um Verbindungsinformationen aus dem Code in Konfigurationsdateien auszulagern:

```rust
use ecat_data_redis::{RedisCache, RedisConfig};
use ecat_data_sqlx::{SqlxClient, SqlxConfig};
use ecat_data_clickhouse::{ClickhouseClient, ClickhouseConfig};

// Aus der Konfigurationsdatei laden (JSON oder YAML)
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

**Referenz der Konfigurationsfelder**:

| Backend | Config | Felder | Beispielwert |
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
| Memcached | `MemcachedConfig` | `username`?, `password`? (reservierte Felder) | — |
| TDengine | `TdengineConfig` | `base_url`, `username`, `password`, `database`? | `http://localhost:6041` |
| MongoDB | `MongoConfig` | `url`, `database`, `tls`? | `mongodb://localhost:27017`, `app` |
| S3 | `S3Config` | `endpoint`, `region`, `access_key`, `secret_key`, `tls`? | `http://localhost:9000`, `us-east-1` |

> Alle Backend-Configs unterstützen das optionale Feld `tls` (`TlsClientConfig`) zur Konfiguration der TLS-Client-Zertifikatsauthentifizierung. Details siehe [Tutorial zur Datenbankkonfiguration](database-config-tutorial.md).

## Projektstruktur

```
e-cat/
├── ecat/                       # Kern: App-Lebenszyklus
├── ecat-transport/             # Transportabstraktion (Server-Trait)
├── ecat-transport-http/        # axum-Implementierung
├── ecat-transport-grpc/        # tonic-Implementierung
├── ecat-middleware/            # tower::Layer-Middleware
├── ecat-protos/                # Protobuf-Definitionen
├── ecat-errors/                # Fehlercodesystem
├── ecat-metadata/              # Metadatenübertragung
├── ecat-encoding/              # Serialisierungsabstraktion
├── ecat-logging/               # tracing-Integration
├── ecat-registry/              # Service-Registrierung/Discovery
├── ecat-config/                # Konfigurationsverwaltung
├── ecat-metrics/               # Prometheus-Integration
├── ecat-data/                  # Datenzugriffs-Traits
├── ecat-security/              # Angriffserkennung (security-rust)
├── ecat-cli/                   # CLI-Werkzeuge
├── ecat-health/                # Health-Checks (/health /ready)
├── ecat-auth/                  # Auth-Middleware (JWT / API Key)
├── ecat-client/                # HTTP-Client für Dienstkommunikation
├── ecat-circuit-breaker/       # Leistungsschalter (Tower Layer)
├── ecat-registry-consul/       # Consul-Service-Registrierung
├── ecat-config-remote/         # Consul-KV-Fernkonfiguration
├── ecat-data-redis/            # Redis-Cache-Implementierung
├── ecat-mq/                    # Message-Queue-Abstraktion
├── ecat-events/                # Event-Bus (lokal + remote)
├── ecat-testing/               # Integrationstest-Werkzeuge
├── ecat-openapi/               # OpenAPI-spec-Generierung
├── ecat-bench/                 # Performance-Benchmarks
├── ecat-tracing/               # Verteilte Ablaufverfolgung (trace_id-Injektion/-Extraktion)
├── ecat-registry-etcd/         # etcd-Service-Registrierung
├── ecat-mq-kafka/              # Kafka-Message-Queue-Adapter
├── ecat-data-opensearch/       # OpenSearch-Such-Backend
├── ecat-data-influxdb/         # InfluxDB-Zeitreihen-Backend
├── ecat-graphql/               # GraphQL-Endpoint
├── ecat-data-elasticsearch/    # Elasticsearch-Such-Backend
├── ecat-data-clickhouse/       # ClickHouse-OLAP-Backend
├── ecat-data-sqlx/             # RDBMS-Backend (SQLite/PG/MySQL/TiDB)
├── ecat-data-memcached/        # Memcached-Cache-Backend (Speicherimplementierung)
├── ecat-data-neo4j/            # Neo4j-Graph-Backend
├── ecat-data-nebulagraph/      # NebulaGraph-Graph-Backend
├── ecat-data-arangodb/         # ArangoDB-Graph-Backend
├── ecat-data-iotdb/            # IoTDB-Zeitreihen-Backend
├── ecat-data-questdb/          # QuestDB-Zeitreihen-Backend
├── ecat-transport-ws/          # WebSocket-Transport
├── ecat-versioning/            # API-Versions-Routing
├── ecat-tls/                   # TLS-Zertifikatskonfiguration und -autogenerierung
├── ecat-deploy/                # Docker / K8s / Helm / CI/CD
├── ecat-lock/                  # Verteilte-Lock-Abstraktion (Redis-Implementierung)
├── ecat-scheduler/             # tokio-Timer-Task-Scheduling
├── ecat-tracing-otlp/          # OpenTelemetry-OTLP-Tracing-Export
├── ecat-data-tdengine/         # TDengine-Zeitreihen-Backend
├── ecat-data-mongodb/          # MongoDB-Dokumenten-Backend
├── ecat-data-s3/               # S3-/MinIO-Objektspeicher-Backend
├── ecat-mq-rabbitmq/           # RabbitMQ-Nachrichten-Backend
├── ecat-mq-mqtt/               # MQTT-Nachrichten-Backend
├── ecat-mq-nats/               # NATS-Nachrichten-Backend
├── config/                     # Beispiel-Konfigurationsdateien
├── docs/                       # Designdokumente und Ökosystemplanung
└── examples/                   # Beispielprojekte
```

## Schnellstart

### Voraussetzungen

- Rust 1.85+ (stable-Toolchain, Edition 2024 erforderlich)
- [protoc](https://github.com/protocolbuffers/protobuf) (Protocol-Buffers-Compiler)

### CLI installieren

```bash
cargo install ecat-cli
```

### Dienst erstellen

```bash
# Projekt-Gerüst generieren
ecat new helloworld
cd helloworld

# proto-Definition hinzufügen
ecat proto add proto/service.proto

# Client- und Servercode generieren (tonic-build build.rs, ergänzt Cargo.toml-Abhängigkeiten automatisch)
ecat proto client proto/service.proto
ecat proto server proto/service.proto -t internal/service

# Im Entwicklungsmodus ausführen
ecat run

# Änderungen an src/ überwachen und automatisch neu starten
ecat run --watch

# Alle ecat-*-Abhängigkeiten aktualisieren
ecat upgrade
```

`http://localhost:8000/helloworld/ecat` aufrufen.

### Codebeispiel

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

    app.run().await?; // blockiert bis SIGTERM/SIGINT
    Ok(())
}
```

### Aggregations-Crate (ecat)

`ecat` bietet feature-gated Re-Export-Einstiegspunkte — nur die benötigten Komponenten aktivieren:

```rust
use ecat::transport_http::HttpServer;   // feature "http" (Standard)
use ecat::middleware::RecoveryLayer;     // feature "middleware"
use ecat::auth::JwtAuthLayer;            // feature "auth"
use ecat::data::redis::RedisCache;       // feature "redis"
```

Standard-Features = `http+grpc`; mit `--no-default-features --features <Komponente>` lässt sich der Abhängigkeitsbaum schlank halten. Vollständige Feature-Liste: `http` `grpc` `middleware` `auth` `client` `events` `metrics` `tracing` `circuit-breaker` `consul` `remote` `redis`.

### Middleware

```rust
use tower::ServiceBuilder;
use ecat_middleware::{RecoveryLayer, TracingLayer, LoggingLayer, TimeoutLayer};
use ecat_circuit_breaker::CircuitBreakerLayer;
use ecat_security::SecurityLayer;
use ecat_auth::JwtAuthLayer;
use std::time::Duration;

// JWT-Schlüssel muss ≥32 Bytes sein; iss/aud-Claims können verkettet erzwungen werden (optional, Standard: keine Prüfung):
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

> Hinweis: `ecat_middleware::TracingLayer` injiziert keine trace_id; für die Request-bezogene trace_id-Injektion bitte `ecat_tracing::TracingLayer::new()` verwenden.

```rust
// Metriken: Request-Zähler und Latenz in das globale Registry aufzeichnen (geteilt mit dem /metrics-Endpunkt)
use ecat_metrics::MetricsLayer;
let app = Router::new().route("/hello", get(hello)).layer(MetricsLayer::new());
// Metriknamen: ecat_http_requests_total / ecat_http_request_duration_seconds
// (Labels method/path/status). Bei Hochkardinalität wie IDs im Pfad bitte
// MetricsLayer::new().with_path_fn(...) zur Normalisierung verwenden, um Metrik-Kardinalitätsexplosion zu vermeiden.

// Retry: exponentieller Backoff; ⚠️ nur für idempotente Requests (GET/HEAD/PUT/DELETE) sicher
use ecat_middleware::RetryLayer;
let retry = RetryLayer::new(3, Duration::from_secs(1), Duration::from_secs(30)); // insgesamt 3 Versuche inkl. erstem
// Benutzerdefinierte Retry-Regel: RetryLayer::new(3, ...).with_rule(MyRule)  // nach Statuscode/Response-Inhalt entscheiden

// Validierung: Header/Parameter vor dem Routing prüfen, bei Fehlschlag kurzes JSON-Fehler-Response (Standard 400, mit with_status z. B. 422)
use ecat_middleware::{ValidateLayer, ValidateError};
let validate = ValidateLayer::from_fn(|req: &http::Request<axum::body::Body>| {
    if req.headers().contains_key("x-api-key") {
        Ok(())
    } else {
        Err(ValidateError::new("missing x-api-key").with_status(422))
    }
});

// CORS: ecat-middleware muss mit dem "cors"-Feature kompiliert werden
use ecat_middleware::{CorsLayer, AllowOrigin};
let cors = CorsLayer::new().allow_origin(AllowOrigin::any());
```

### Fehlerbehandlung

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

## Implementierungsphasen

| Phase | Status | Inhalt |
|------|------|------|
| Phase 1 | ✅ Abgeschlossen | Projekt-Gerüst, protos, errors, metadata, encoding, logging |
| Phase 2 | ✅ Abgeschlossen | Transport-Ebene (HTTP + gRPC) |
| Phase 3 | ✅ Abgeschlossen | Middleware-System (Recovery/Tracing/Logging/Timeout) |
| Phase 4 | ✅ Abgeschlossen | App-Lebenszyklusverwaltung |
| Phase 5 | ✅ Abgeschlossen | Registry, Config, Metrics |
| Phase 5.5 | ✅ Abgeschlossen | Datenzugriffsebene (Traits + sqlx-Backend) |
| Phase 6 | ✅ Abgeschlossen | CLI-Werkzeugkette (new/proto/run/build) |
| Phase 7 | ✅ Abgeschlossen | README, Beispiele (helloworld), Designdokumente |
| Phase 8 | ✅ Abgeschlossen | Angriffserkennungsintegration (security-rust, ecat-security) |
| Phase 9 | ✅ Abgeschlossen | Ökosystem Phase 1 (health / client / circuit-breaker / auth / registry-consul) |
| Phase 10 | ✅ Abgeschlossen | Ökosystem Phase 2 (redis / mq / events / config-remote) |
| Phase 11 | ✅ Abgeschlossen | Ökosystem Phase 3 (testing / deploy / bench / openapi) |
| Phase 12 | ✅ Abgeschlossen | Kommunikation und Sicherheit gestärkt (gRPC-Client / OAuth2 / mTLS / verteilte Ablaufverfolgung) |
| Phase 13 | ✅ Abgeschlossen | Daten-Backends ergänzt (etcd / Kafka / OpenSearch / InfluxDB) |
| Phase 14 | ✅ Abgeschlossen | Betrieb und Erlebnis (WebSocket / API-Versionsverwaltung / Helm / CI/CD) |
| Phase 15 | ✅ Abgeschlossen | Ökosystem-Erweiterung v2 (echtes Kafka / RabbitMQ / MQTT / NATS / MongoDB / S3 / TDengine / OTLP / verteilte Locks / Scheduling / CLI watch+upgrade) |
| Phase 16 | ✅ Abgeschlossen | Wartungsstärkung v2.4 (M1 MetricsLayer / M2 RetryLayer / M3 ValidateLayer / M4 CORS / U1 Aggregations-Crate ecat / U2 examples / OAuth2-Token-Hash / CVE-Tracking) |

## Bekannte Einschränkungen

- **GraphQL-Parser (ecat-graphql)**: unterstützt Feldargumente und verschachtelte Selections (`query_field`/`mutation_field`-Rich-Resolvers können auf `args`/`variables`/`selection` zugreifen); Aliase, Fragmente und mehrere Top-Level-Felder werden weiterhin nicht unterstützt — bitte nicht als generischen GraphQL-Endpoint freigeben.
- **OAuth2-Introspections-Cache (ecat-auth)**: Cache-Key ist der SHA-256-Hash des Tokens (kein Klartext-Token gespeichert); Cache-Werte werden per Whitelist gefiltert (Standard: sub/exp/iat/role + extra iss/aud/scope/roles, `cache_claims_whitelist` konfigurierbar; bei Miss werden weiterhin vollständige Claims zurückgegeben, nur der Cache-Wert wird gefiltert); abgelaufene TTL-Einträge werden beim Schreiben aktiv entfernt (Standard-TTL 300s).
- **Kafka-Offset (ecat-mq-kafka)**: Standardmäßig `enable.auto.commit=false` ohne manuelles Commit — nach einem Neustart wird ab dem Partitionsende (latest) neu gelesen, während des Ausfalls erzeugte Nachrichten werden übersprungen; at-least-once-Semantik (Neustart fährt ab dem letzten Committed Point fort) erfordert explizites `auto_commit=true`.

## Designziele

| # | Ziel | Beschreibung |
|---|------|------|
| 1 | **Kratos-Angleichung** | API-first-, pluggbare, einheitliche Abstraktionsphilosophie von Kratos beibehalten |
| 2 | **Rust-idiomatisch** | tower::Service, generische Traits, Zero-Cost-Abstraktionen wiederverwenden; kein „Go in Rust" |
| 3 | **Typsicherheit** | Fehler zur Compilezeit abfangen, Protobuf-Definitionen vollständig stark typisiert |
| 4 | **Pluggable** | Registry, Config, Logging, Encoding — alles über Traits abstrahiert |
| 5 | **Vollständige Werkzeugkette** | CLI unterstützt Projekt-Gerüst, proto-Codegenerierung, Entwicklungsausführung |
| 6 | **Performance zuerst** | Zero-Cost-Abstraktionen + Async-Runtime |
| 7 | **Beobachtbar** | tracing + Prometheus out of the box |
| 8 | **Vollständiges Ökosystem** | Clients, Leistungsschalter, Auth, Health-Checks, Registry-Backends |

## Technische Hinweise

### Warum tower::Service

[`tower::Service`](https://docs.rs/tower/latest/tower/trait.Service.html) ist das Äquivalent zu `http.Handler` im asynchronen Rust-Ökosystem. Sowohl axum als auch tonic bauen auf tower auf, daher benötigt e-cat kein eigenes Middleware-Trait — tower::Layer-Implementierungen direkt bereitzustellen erreicht denselben Effekt wie Kratos-Middleware, mit null Adapter-Overhead.

### Warum ein Cargo Workspace

Konsistent mit dem modularen Design von Kratos. Alle `ecat-*`-Crates werden im Workspace mit synchronisierten Versionen veröffentlicht (aktuell 3.0.2), jeweils unabhängig kompiliert, Nutzer binden nach Bedarf ein. Kern-Crates halten die Abhängigkeiten minimal, Contrib-Crates bieten optionale Integrationen.

### Warum prost (statt protobuf-rs)

prost ist die am weitesten verbreitete Protobuf-Implementierung in der Rust-Community, generiert zur Compilezeit typsicheren Code und ist tief in tonic integriert.

## Designdokumente

- [Designspezifikation](../../../docs/superpowers/specs/2026-07-29-ecat-framework-design.md)
- [Implementierungsplan](../../../docs/superpowers/plans/2026-07-29-ecat-framework.md)
- [Ökosystemplan v1](ecosystem-plan.md) (abgeschlossen)
- [Ökosystemplan v2](ecosystem-plan-v2.md) (abgeschlossen)
- [Ökosystemplan v3](ecosystem-plan-v3.md) (endgültige Bewertung)
- [API-Referenz](api.md)
- [Auditbericht r5](audit-report-2026-08-01-r5.md) (2026-08-01)
- [Tutorial zur Datenbankkonfiguration](database-config-tutorial.md)
- [CVE-Tracking der Abhängigkeiten](dependency-cve-tracking.md)
- [Tutorial zur TLS-Zertifikatsauthentifizierung](tls-certificate-tutorial.md)
- [Beispiel-Konfigurationsdatei](../../../config/databases.example.yaml)

## Unterstützung

Wir freuen uns über Unterstützung für dieses Projekt!

| WeChat Pay | Alipay |
|:---:|:---:|
| <img src="weixinpay.png" width="130" height="130" alt="WeChat Pay"> | <img src="alipay.png" width="130" height="130" alt="Alipay"> |

### Überweisung (Banküberweisung)

| Feld | Informationen |
|------|------|
| Beneficiary Name | WANG KEXUN |
| Account Number | 881015918251 |
| Beneficiary Bank | ZA Bank Limited |
| SWIFT Code | AABLHKHHXXX |
| Bank Code | 387 |
| Bank Address | Core F, Cyberport 3, 100 Cyberport Road, Hong Kong |

> **Korrespondenzbank für internationale Überweisungen (falls erforderlich)**: Dies sind die Informationen der Korrespondenzbank (Zwischenbank), nicht die der Empfängerbank. Bitte bei der überweisenden Bank erfragen, ob diese Angaben benötigt werden.
>
> - Für Überweisungen in Hongkong-Dollar, Renminbi und US-Dollar: **Citibank N.A. Hong Kong** (SWIFT: `CITIHKHXXXX`, Bank Code: 006, Branch: Hong Kong Branch, Branch Code: 391, Adresse: Citibank Tower, Citibank Plaza, 3 Garden Road, Central, Hong Kong)
> - Für Überweisungen in anderen Währungen: **THE BANK OF NEW YORK MELLON** (SWIFT: `IRVTUS3NXXX`, Adresse: 240 GREENWICH STREET, NEW YORK, United States)

## Lizenz

Apache-2.0
