<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# Ecat

[简体中文](../../../README.md) | [English](../../../README.en.md) | [日本語](../ja/README.md) | [한국어](../ko/README.md) | [Русский](../ru/README.md) | [Deutsch](../de/README.md) | [Français](../fr/README.md) | [Español](../es/README.md) | [Português](../pt/README.md) | **हिन्दी** | [العربية](../ar/README.md) | [বাংলা](../bn/README.md) | [Bahasa Indonesia](../id/README.md)

Ecat का चीनी नाम: एक बिल्ली (一只猫)

**एक बिल्ली** एक Rust माइक्रोसर्विस फ्रेमवर्क है जो [go-kratos/kratos](https://github.com/go-kratos/kratos) v3 के समकक्ष है (v3.0.2 · 51 crates)।

यह API-first विकास अनुभव, प्लग-योग्य घटक आर्किटेक्चर, एकीकृत HTTP/gRPC मिडलवेयर एब्स्ट्रैक्शन और संपूर्ण CLI टूलचेन प्रदान करता है। Kratos से परिचित डेवलपर्स बिना किसी रुकावट के आरंभ कर सकते हैं, साथ ही Rust की टाइप-सुरक्षा, शून्य-लागत एब्स्ट्रैक्शन और अत्यधिक प्रदर्शन का पूरा लाभ उठा सकते हैं।

<p align="center">
  <img src="e-cat.svg" alt="Ecat प्रोजेक्ट पालतू (डायनामिक)" width="220" />
</p>

## डिज़ाइन आर्किटेक्चर

```
┌──────────────────────────────────────────────────────────────┐
│                         ecat-cli                             │
│        (new │ proto │ run --watch │ build │ upgrade)         │
├──────────────────────────────────────────────────────────────┤
│                     ecat (एप्लिकेशन लाइफसाइकिल)                │
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
│                         data परत                              │
│     ────────────────────────────────────────────────          │
│     rdbms:   SQLite / PostgreSQL / MySQL / TiDB              │
│     cache:   Redis ✓                                         │
│     config:  remote (Consul KV)                              │
│     registry: consul                                         │
├──────────────────────────────────────────────────────────────┤
│                       ecat-protos                             │
│     (साझा .proto परिभाषाएँ: errors, metadata, ...)            │
└──────────────────────────────────────────────────────────────┘
```

### अनुरोध प्रोसेसिंग प्रवाह

```
क्लाइंट अनुरोध
  │
  ├─ HTTP 0.0.0.0:8000 ──→ axum::Router ──┐
  │                                        │
  └─ gRPC 0.0.0.0:9000 ──→ tonic::Server ─┤
                                      │
                              ┌───────┴───────┐
                              │   Middleware   │
                              │   ──────────   │
                              │ 1. Recovery    │   panic पकड़ता है
                              │ 2. Tracing     │   trace_id इंजेक्ट करता है
                              │ 3. Logging     │   अनुरोध लॉग
                              │ 4. Auth        │   प्रमाणीकरण/प्राधिकरण
                              │ 5. Metrics     │   मेट्रिक्स संग्रह
                              │ 6. Security    │   हमले की पहचान
                              │ 7. CircuitBrk  │   सर्किट ब्रेकर सुरक्षा
                              └───────┬───────┘
                                      │
                              ┌───────┴───────┐
                              │    Handler     │   उपयोगकर्ता व्यावसायिक तर्क
                              │ (tower::Service)│
                              └───────┬───────┘
                                      │
                              ┌───────┴───────┐
                              │   Response     │   एन्कोडिंग/सीरियलाइज़ेशन
                              │ JSON/Protobuf  │
                              └───────────────┘
```

## सुविधाएँ

- **API-first**：Protobuf से API, त्रुटि कोड, मेटाडेटा परिभाषित करें; prost + tonic-build कोड जनरेशन
- **दोहरा प्रोटोकॉल समर्थन**：HTTP (axum) और gRPC (tonic) समान tower::Layer मिडलवेयर सेट साझा करते हैं
- **प्लग-योग्य आर्किटेक्चर**：Registry, Config, Logging, Encoding सभी trait के माध्यम से एब्स्ट्रैक्ट किए गए हैं, डिफ़ॉल्ट रूप से प्रोडक्शन-रेडी कार्यान्वयन उपलब्ध हैं
- **मिडलवेयर प्रणाली**：अंतर्निहित Recovery, Tracing, Logging, Timeout, RateLimit, Security, CircuitBreaker, MetricsLayer, RetryLayer, ValidateLayer, CORS (cors feature); tower::ServiceBuilder द्वारा संयोजित
- **एप्लिकेशन लाइफसाइकिल**：Builder पैटर्न से App निर्माण, मल्टी-सर्वर समवर्ती प्रारंभ, SIGTERM/SIGINT सिग्नल हैंडलिंग, start/stop लाइफसाइकिल हुक
- **टाइप-सुरक्षा**：protobuf-आधारित त्रुटि कोड प्रणाली, कंपाइल-टाइम HTTP स्टेटस कोड मैपिंग
- **अवलोकनीयता**：tracing + Prometheus + Health एंडपॉइंट (/health、/ready)
- **हमले की पहचान**：SecurityLayer स्वचालित रूप से SQL इंजेक्शन, XSS, SSRF आदि हमले पैटर्न का पता लगाता है और उच्च-जोखिम वाले अनुरोधों को ब्लॉक करता है
- **सेवा-दर-सेवा संचार**：HttpClient सेवा खोज और लोड बैलेंसिंग के साथ एकीकृत, CircuitBreaker सर्किट ब्रेकर सुरक्षा
- **प्रमाणीकरण/प्राधिकरण**：JWT / API Key प्रमाणीकरण मिडलवेयर, Claims अनुरोध संदर्भ में पास की जाती हैं
- **संदेश और इवेंट**：MessageQueue trait + EventBus स्थानीय/रिमोट Pub/Sub
- **वितरित ट्रेसिंग**：अनुरोध span, trace_id इंजेक्शन/निष्कर्षण
- **gRPC क्लाइंट**：GrpcClient सेवा खोज और लोड बैलेंसिंग के साथ एकीकृत
- **बहु-प्रोटोकॉल**：HTTP、gRPC、WebSocket、GraphQL एकीकृत रूटिंग
- **बहु-डेटा स्रोत**：RDBMS (SQLite/PG/MySQL/TiDB)、कैश (Redis/Memcached)、खोज (OpenSearch/Elasticsearch)、ग्राफ (Neo4j/NebulaGraph/ArangoDB)、टाइम-सीरीज़ (InfluxDB/IoTDB/QuestDB/TDengine)、दस्तावेज़ (MongoDB)、ऑब्जेक्ट स्टोरेज (S3/MinIO)

### Kratos अवधारणा मैपिंग

| Kratos (Go) | e-cat (Rust) | स्पष्टीकरण |
|-------------|-------------|------|
| `kratos.New()` | `App::builder()` | Builder पैटर्न |
| `http.Handler` | `tower::Service` | Rust इकोसिस्टम मानक trait |
| `http.Server` | `axum::Router` | समुदाय का मुख्यधारा HTTP फ्रेमवर्क |
| `grpc.Server` | `tonic::transport::Server` | सबसे परिपक्व gRPC कार्यान्वयन |
| `proto generate` | `prost + tonic-build` | समुदाय मानक protobuf |
| `registry.Discovery` | `Registry` trait | प्लग-योग्य रजिस्ट्री/डिस्कवरी |
| `config.Source` | `ConfigSource` trait | बहु-स्रोत कॉन्फ़िगरेशन लोडिंग |

## तकनीकी स्टैक

| घटक | चयन |
|------|------|
| एसिंक रनटाइम | **tokio** |
| HTTP | **axum** |
| gRPC | **tonic** |
| Protobuf | **prost + tonic-build** |
| मिडलवेयर | **tower::Service / Layer** |
| लॉग/ट्रेसिंग | **tracing + trace_id propagation** |
| मेट्रिक्स | **prometheus** |
| सीरियलाइज़ेशन | **serde + prost** |
| हमले की पहचान | **security-rust** |
| RDBMS | **sqlx** |
| Redis | **redis-rs** |
| JWT | **jsonwebtoken** |
| HTTP क्लाइंट | **reqwest** |
| CLI | **clap** |

## समर्थित डेटाबेस

| श्रेणी | डेटाबेस | Crate | स्थिति |
|------|--------|-------|------|
| RDBMS | SQLite | `ecat-data-sqlx` | ✅ लागू |
| RDBMS | PostgreSQL | `ecat-data-sqlx` | ✅ लागू |
| RDBMS | MySQL | `ecat-data-sqlx` | ✅ लागू |
| RDBMS | TiDB | `ecat-data-sqlx` | ✅ लागू |
| कैश | Redis | `ecat-data-redis` | ✅ लागू |
| खोज | OpenSearch | `ecat-data-opensearch` | ✅ लागू |
| खोज | Elasticsearch | `ecat-data-elasticsearch` | ✅ लागू |
| कैश | Memcached | `ecat-data-memcached` | ⚠️ मेमोरी कार्यान्वयन (प्रोडक्शन नहीं, स्थायी कैश के लिए उपयोग न करें) |
| OLAP | ClickHouse | `ecat-data-clickhouse` | ✅ लागू |
| ग्राफ | Neo4j | `ecat-data-neo4j` | ✅ REST API |
| ग्राफ | NebulaGraph | `ecat-data-nebulagraph` | ✅ REST API |
| ग्राफ | ArangoDB | `ecat-data-arangodb` | ✅ REST API |
| टाइम-सीरीज़ | InfluxDB | `ecat-data-influxdb` | ✅ HTTP API |
| टाइम-सीरीज़ | Apache IoTDB | `ecat-data-iotdb` | ✅ REST API |
| टाइम-सीरीज़ | QuestDB | `ecat-data-questdb` | ✅ HTTP API |
| टाइम-सीरीज़ | TDengine | `ecat-data-tdengine` | ✅ REST API |
| दस्तावेज़ | MongoDB | `ecat-data-mongodb` | ✅ नेटिव ड्राइवर |
| ऑब्जेक्ट स्टोरेज | S3 / MinIO | `ecat-data-s3` | ✅ reqwest+rustls |

> सभी डेटा बैकएंड एकीकृत trait एब्स्ट्रैक्शन (`RdbmsClient` / `Cache` / `SearchClient` / `GraphClient` / `TsdbClient` / `DocumentClient` / `StorageClient`) के माध्यम से उपलब्ध हैं, आवश्यकता अनुसार संबंधित contrib crate आयात करें। प्रत्येक बैकएंड एक `XxxConfig` स्ट्रक्चर (`#[derive(Deserialize)]`) प्रदान करता है, जो JSON/YAML कॉन्फ़िगरेशन फ़ाइल से कनेक्शन जानकारी लोड करने का समर्थन करता है।

> **कंस्ट्रक्टर नामकरण परंपरा**：मैसेज क्यू crates (`ecat-mq-*`) का मुख्य कंस्ट्रक्टर एकीकृत रूप से `connect` है (जैसे `KafkaMq::connect(brokers)`、`MqttMq::connect(url)`), साथ ही कॉन्फ़िगरेशन से लोड करने के लिए `from_config` प्रदान करता है; डेटा बैकएंड crates (`ecat-data-*`) का अधिकांश मुख्य कंस्ट्रक्टर `new` है, अपवाद: `ecat-data-redis` / `ecat-data-sqlx` `connect` का उपयोग करते हैं, `ecat-data-mongodb` / `ecat-data-s3` केवल `from_config` प्रदान करते हैं। यह मौजूदा परंपरा है, अनिवार्य एकीकरण नहीं (ब्रेकिंग चेंज से बचने के लिए); 3.0 विंडो में एकीकरण का मूल्यांकन किया जा सकता है।

### डेटाबेस कॉन्फ़िगरेशन उदाहरण

प्रत्येक डेटा बैकएंड `XxxConfig` स्ट्रक्चर और `from_config()` विधि प्रदान करता है, कनेक्शन जानकारी को कोड से कॉन्फ़िगरेशन फ़ाइल में स्थानांतरित करता है:

```rust
use ecat_data_redis::{RedisCache, RedisConfig};
use ecat_data_sqlx::{SqlxClient, SqlxConfig};
use ecat_data_clickhouse::{ClickhouseClient, ClickhouseConfig};

// कॉन्फ़िगरेशन फ़ाइल से लोड करें (JSON या YAML)
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

**कॉन्फ़िगरेशन फ़ील्ड संदर्भ**:

| बैकएंड | Config | फ़ील्ड | उदाहरण मान |
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
| Memcached | `MemcachedConfig` | `username`?, `password`? (आरक्षित फ़ील्ड) | — |
| TDengine | `TdengineConfig` | `base_url`, `username`, `password`, `database`? | `http://localhost:6041` |
| MongoDB | `MongoConfig` | `url`, `database`, `tls`? | `mongodb://localhost:27017`, `app` |
| S3 | `S3Config` | `endpoint`, `region`, `access_key`, `secret_key`, `tls`? | `http://localhost:9000`, `us-east-1` |

> सभी बैकएंड Config वैकल्पिक `tls` फ़ील्ड (`TlsClientConfig`) का समर्थन करते हैं, जो TLS क्लाइंट प्रमाणपत्र प्रमाणीकरण कॉन्फ़िगर करने के लिए है। विवरण के लिए देखें [डेटाबेस कॉन्फ़िगरेशन ट्यूटोरियल](database-config-tutorial.md)。

## प्रोजेक्ट संरचना

```
e-cat/
├── ecat/                       # कोर: App लाइफसाइकिल
├── ecat-transport/             # ट्रांसपोर्ट एब्स्ट्रैक्शन (Server trait)
├── ecat-transport-http/        # axum कार्यान्वयन
├── ecat-transport-grpc/        # tonic कार्यान्वयन
├── ecat-middleware/            # tower::Layer मिडलवेयर
├── ecat-protos/                # Protobuf परिभाषाएँ
├── ecat-errors/                # त्रुटि कोड प्रणाली
├── ecat-metadata/              # मेटाडेटा संचरण
├── ecat-encoding/              # सीरियलाइज़ेशन एब्स्ट्रैक्शन
├── ecat-logging/               # tracing एकीकरण
├── ecat-registry/              # सेवा रजिस्ट्री/डिस्कवरी
├── ecat-config/                # कॉन्फ़िगरेशन प्रबंधन
├── ecat-metrics/               # Prometheus एकीकरण
├── ecat-data/                  # डेटा एक्सेस traits
├── ecat-security/              # हमले की पहचान (security-rust)
├── ecat-cli/                   # CLI उपकरण
├── ecat-health/                # हेल्थ चेक (/health /ready)
├── ecat-auth/                  # प्रमाणीकरण मिडलवेयर (JWT / API Key)
├── ecat-client/                # सेवा-दर-सेवा HTTP क्लाइंट
├── ecat-circuit-breaker/       # सर्किट ब्रेकर (Tower Layer)
├── ecat-registry-consul/       # Consul सेवा रजिस्ट्री
├── ecat-config-remote/         # Consul KV रिमोट कॉन्फ़िगरेशन
├── ecat-data-redis/            # Redis कैश कार्यान्वयन
├── ecat-mq/                    # मैसेज क्यू एब्स्ट्रैक्शन
├── ecat-events/                # इवेंट बस (स्थानीय + रिमोट)
├── ecat-testing/               # एकीकरण परीक्षण उपकरण
├── ecat-openapi/               # OpenAPI spec जनरेशन
├── ecat-bench/                 # प्रदर्शन बेंचमार्क
├── ecat-tracing/               # वितरित ट्रेसिंग (trace_id इंजेक्शन/निष्कर्षण)
├── ecat-registry-etcd/         # etcd सेवा रजिस्ट्री
├── ecat-mq-kafka/              # Kafka मैसेज क्यू एडाप्टर
├── ecat-data-opensearch/       # OpenSearch खोज बैकएंड
├── ecat-data-influxdb/         # InfluxDB टाइम-सीरीज़ बैकएंड
├── ecat-graphql/               # GraphQL endpoint
├── ecat-data-elasticsearch/    # Elasticsearch खोज बैकएंड
├── ecat-data-clickhouse/       # ClickHouse OLAP बैकएंड
├── ecat-data-sqlx/             # RDBMS बैकएंड (SQLite/PG/MySQL/TiDB)
├── ecat-data-memcached/        # Memcached कैश बैकएंड (मेमोरी कार्यान्वयन)
├── ecat-data-neo4j/            # Neo4j ग्राफ बैकएंड
├── ecat-data-nebulagraph/      # NebulaGraph ग्राफ बैकएंड
├── ecat-data-arangodb/         # ArangoDB ग्राफ बैकएंड
├── ecat-data-iotdb/            # IoTDB टाइम-सीरीज़ बैकएंड
├── ecat-data-questdb/          # QuestDB टाइम-सीरीज़ बैकएंड
├── ecat-transport-ws/          # WebSocket transport
├── ecat-versioning/            # API संस्करण रूटिंग
├── ecat-tls/                   # TLS प्रमाणपत्र कॉन्फ़िगरेशन और स्वतः जनरेशन
├── ecat-deploy/                # Docker / K8s / Helm / CI/CD
├── ecat-lock/                  # वितरित लॉक एब्स्ट्रैक्शन (Redis कार्यान्वयन)
├── ecat-scheduler/             # tokio टाइमर कार्य शेड्यूलिंग
├── ecat-tracing-otlp/          # OpenTelemetry OTLP ट्रेसिंग एक्सपोर्ट
├── ecat-data-tdengine/         # TDengine टाइम-सीरीज़ बैकएंड
├── ecat-data-mongodb/          # MongoDB दस्तावेज़ बैकएंड
├── ecat-data-s3/               # S3 / MinIO ऑब्जेक्ट स्टोरेज बैकएंड
├── ecat-mq-rabbitmq/           # RabbitMQ मैसेज बैकएंड
├── ecat-mq-mqtt/               # MQTT मैसेज बैकएंड
├── ecat-mq-nats/               # NATS मैसेज बैकएंड
├── config/                     # कॉन्फ़िगरेशन उदाहरण फ़ाइलें
├── docs/                       # डिज़ाइन दस्तावेज़ और इकोसिस्टम योजना
└── examples/                   # उदाहरण प्रोजेक्ट
```

## त्वरित आरंभ

### पूर्वापेक्षाएँ

- Rust 1.85+ (stable टूलचेन, edition 2024 आवश्यक)
- [protoc](https://github.com/protocolbuffers/protobuf) (Protocol Buffers कंपाइलर)

### CLI इंस्टॉल करें

```bash
cargo install ecat-cli
```

### सेवा बनाएं

```bash
# स्कैफ़ोल्ड से प्रोजेक्ट जनरेट करें
ecat new helloworld
cd helloworld

# proto परिभाषा जोड़ें
ecat proto add proto/service.proto

# क्लाइंट और सर्वर कोड जनरेट करें (tonic-build build.rs, Cargo.toml निर्भरताएँ स्वतः पूरी होती हैं)
ecat proto client proto/service.proto
ecat proto server proto/service.proto -t internal/service

# डेवलपमेंट मोड में चलाएँ
ecat run

# src/ परिवर्तन देखकर स्वतः रीस्टार्ट करें
ecat run --watch

# सभी ecat-* निर्भरताएँ अपडेट करें
ecat upgrade
```

`http://localhost:8000/helloworld/ecat` पर जाएँ।

### कोड उदाहरण

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

    app.run().await?; // SIGTERM/SIGINT तक ब्लॉक करता है
    Ok(())
}
```

### एग्रीगेट crate (ecat)

`ecat` feature-gated re-export एंट्री प्रदान करता है — केवल आवश्यक घटक सक्षम करें:

```rust
use ecat::transport_http::HttpServer;   // feature "http" (डिफ़ॉल्ट)
use ecat::middleware::RecoveryLayer;     // feature "middleware"
use ecat::auth::JwtAuthLayer;            // feature "auth"
use ecat::data::redis::RedisCache;       // feature "redis"
```

डिफ़ॉल्ट features = `http+grpc`; `--no-default-features --features <घटक>` से निर्भरता ट्री हल्का करें। पूर्ण feature सूची: `http` `grpc` `middleware` `auth` `client` `events` `metrics` `tracing` `circuit-breaker` `consul` `remote` `redis`।

### मिडलवेयर

```rust
use tower::ServiceBuilder;
use ecat_middleware::{RecoveryLayer, TracingLayer, LoggingLayer, TimeoutLayer};
use ecat_circuit_breaker::CircuitBreakerLayer;
use ecat_security::SecurityLayer;
use ecat_auth::JwtAuthLayer;
use std::time::Duration;

// JWT कुंजी ≥32 बाइट्स होनी चाहिए; iss/aud claims को चेन करके अनिवार्य करें (वैकल्पिक, डिफ़ॉल्ट रूप से जाँच नहीं):
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

> नोट: `ecat_middleware::TracingLayer` trace_id इंजेक्ट नहीं करता; अनुरोध-स्तर trace_id इंजेक्शन के लिए `ecat_tracing::TracingLayer::new()` उपयोग करें।

```rust
// मेट्रिक्स: अनुरोध गणना और विलंबता को ग्लोबल registry में रिकॉर्ड करता है (/metrics endpoint के साथ साझा)
use ecat_metrics::MetricsLayer;
let app = Router::new().route("/hello", get(hello)).layer(MetricsLayer::new());
// मेट्रिक्स नाम: ecat_http_requests_total / ecat_http_request_duration_seconds
// (लेबल method/path/status)। पथ में ID जैसे उच्च-कार्डिनैलिटी परिदृश्यों के लिए
// MetricsLayer::new().with_path_fn(...) से नॉर्मलाइज़ करें, मेट्रिक्स कार्डिनैलिटी विस्फोट से बचें।

// रीट्राई: एक्सपोनेंशियल बैकऑफ़; ⚠️ केवल आइडेम्पोटेंट अनुरोधों (GET/HEAD/PUT/DELETE) के लिए सुरक्षित
use ecat_middleware::RetryLayer;
let retry = RetryLayer::new(3, Duration::from_secs(1), Duration::from_secs(30)); // पहले सहित कुल 3 प्रयास
// कस्टम रीट्राई नियम: RetryLayer::new(3, ...).with_rule(MyRule)  // स्टेटस कोड/प्रतिक्रिया सामग्री के अनुसार निर्णय

// वैलिडेशन: रूट से पहले header/पैरामीटर सत्यापित करें, असफल होने पर JSON त्रुटि लौटाएँ (डिफ़ॉल्ट 400, with_status से 422 आदि बदलें)
use ecat_middleware::{ValidateLayer, ValidateError};
let validate = ValidateLayer::from_fn(|req: &http::Request<axum::body::Body>| {
    if req.headers().contains_key("x-api-key") {
        Ok(())
    } else {
        Err(ValidateError::new("missing x-api-key").with_status(422))
    }
});

// CORS: ecat-middleware में "cors" feature सक्षम करना आवश्यक है
use ecat_middleware::{CorsLayer, AllowOrigin};
let cors = CorsLayer::new().allow_origin(AllowOrigin::any());
```

### त्रुटि प्रबंधन

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

## कार्यान्वयन चरण

| चरण | स्थिति | सामग्री |
|------|------|------|
| Phase 1 | ✅ पूर्ण | प्रोजेक्ट स्केलेटन, protos, errors, metadata, encoding, logging |
| Phase 2 | ✅ पूर्ण | Transport परत (HTTP + gRPC) |
| Phase 3 | ✅ पूर्ण | Middleware प्रणाली (Recovery/Tracing/Logging/Timeout) |
| Phase 4 | ✅ पूर्ण | App लाइफसाइकिल प्रबंधन |
| Phase 5 | ✅ पूर्ण | Registry, Config, Metrics |
| Phase 5.5 | ✅ पूर्ण | Data एक्सेस परत (traits + sqlx बैकएंड) |
| Phase 6 | ✅ पूर्ण | CLI टूलचेन (new/proto/run/build) |
| Phase 7 | ✅ पूर्ण | README, उदाहरण (helloworld), डिज़ाइन दस्तावेज़ |
| Phase 8 | ✅ पूर्ण | हमले की पहचान एकीकरण (security-rust, ecat-security) |
| Phase 9 | ✅ पूर्ण | इकोसिस्टम फेज़ 1 (health / client / circuit-breaker / auth / registry-consul) |
| Phase 10 | ✅ पूर्ण | इकोसिस्टम फेज़ 2 (redis / mq / events / config-remote) |
| Phase 11 | ✅ पूर्ण | इकोसिस्टम फेज़ 3 (testing / deploy / bench / openapi) |
| Phase 12 | ✅ पूर्ण | संचार और सुरक्षा सुदृढ़ीकरण (gRPC क्लाइंट / OAuth2 / mTLS / वितरित ट्रेसिंग) |
| Phase 13 | ✅ पूर्ण | डेटा बैकएंड पूर्ति (etcd / Kafka / OpenSearch / InfluxDB) |
| Phase 14 | ✅ पूर्ण | संचालन और अनुभव (WebSocket / API संस्करण प्रबंधन / Helm / CI/CD) |
| Phase 15 | ✅ पूर्ण | इकोसिस्टम विस्तार v2 (असली Kafka / RabbitMQ / MQTT / NATS / MongoDB / S3 / TDengine / OTLP / वितरित लॉक / शेड्यूलर / CLI watch+upgrade) |
| Phase 16 | ✅ पूर्ण | रखरखाव सुदृढ़ीकरण v2.4 (M1 MetricsLayer / M2 RetryLayer / M3 ValidateLayer / M4 CORS / U1 एग्रीगेट crate ecat / U2 examples / OAuth2 token hash / CVE ट्रैकिंग) |

## ज्ञात सीमाएँ

- **GraphQL पार्सिंग (ecat-graphql)**：फ़ील्ड पैरामीटर और नेस्टेड selection का समर्थन करता है (`query_field`/`mutation_field` रिच रिज़ॉल्वर `args`/`variables`/`selection` तक पहुँच सकते हैं); फिर भी alias, fragment और मल्टी-टॉप-लेवल फ़ील्ड का समर्थन नहीं करता, इसे सामान्य GraphQL endpoint के रूप में उजागर न करें।
- **OAuth2 इंट्रोस्पेक्शन कैश (ecat-auth)**：कैश कुंजी token का SHA-256 hash है (token प्लेनटेक्स्ट संग्रहीत नहीं होता); कैश मान व्हाइटलिस्ट फ़िल्टर से गुज़रता है (डिफ़ॉल्ट रूप से sub/exp/iat/role + extra का iss/aud/scope/roles रखता है, `cache_claims_whitelist` से कॉन्फ़िगर किया जा सकता है; miss होने पर पूर्ण claims लौटाता है, केवल कैश मान फ़िल्टर होता है); TTL समाप्त प्रविष्टियाँ लिखते समय सक्रिय रूप से साफ़ होती हैं (डिफ़ॉल्ट TTL 300s)।
- **Kafka offset (ecat-mq-kafka)**：डिफ़ॉल्ट `enable.auto.commit=false` और कोई मैन्युअल commit नहीं — प्रक्रिया रीस्टार्ट के बाद पार्टिशन के अंत (latest) से फिर से पढ़ता है, डाउनटाइम के दौरान उत्पन्न संदेश छूट जाते हैं; at-least-once सिमेंटिक्स के लिए स्पष्ट रूप से `auto_commit=true` कॉन्फ़िगर करना आवश्यक है (रीस्टार्ट सबसे हाल के commit पॉइंट से जारी रहता है)।

## डिज़ाइन लक्ष्य

| # | लक्ष्य | स्पष्टीकरण |
|---|------|------|
| 1 | **Kratos संरेखण** | Kratos की API-first, प्लग-योग्य, एकीकृत एब्स्ट्रैक्शन अवधारणाएँ बनाए रखना |
| 2 | **Rust इडियोमैटिक** | tower::Service, trait generics, शून्य-लागत एब्स्ट्रैक्शन का पुनः उपयोग; "Go in Rust" नहीं |
| 3 | **टाइप-सुरक्षा** | कंपाइल-टाइम पर त्रुटियाँ पकड़ना, Protobuf परिभाषाएँ पूर्ण रूप से स्ट्रॉन्ग-टाइप्ड |
| 4 | **प्लग-योग्य** | Registry, Config, Logging, Encoding सभी trait के माध्यम से एब्स्ट्रैक्ट किए गए |
| 5 | **संपूर्ण टूलचेन** | CLI प्रोजेक्ट स्कैफ़ोल्डिंग, proto कोड जनरेशन, डेवलपमेंट रनिंग का समर्थन करता है |
| 6 | **प्रदर्शन-प्रथम** | शून्य-लागत एब्स्ट्रैक्शन + एसिंक रनटाइम |
| 7 | **अवलोकनीय** | tracing + Prometheus आउट-ऑफ-द-बॉक्स |
| 8 | **संपूर्ण इकोसिस्टम** | क्लाइंट, सर्किट ब्रेकर, प्रमाणीकरण, हेल्थ चेक, रजिस्ट्री सेंटर बैकएंड |

## तकनीकी नोट्स

### tower::Service क्यों चुना गया

[`tower::Service`](https://docs.rs/tower/latest/tower/trait.Service.html) Rust एसिंक इकोसिस्टम का `http.Handler` समकक्ष है। axum और tonic दोनों tower पर बने हैं, इसलिए e-cat को कस्टम मिडलवेयर trait की आवश्यकता नहीं — सीधे tower::Layer कार्यान्वयन प्रदान करने से Kratos मिडलवेयर के समान प्रभाव मिलता है, शून्य एडाप्टर ओवरहेड के साथ।

### Cargo Workspace क्यों उपयोग किया गया

Kratos की मॉड्यूलर डिज़ाइन के अनुरूप। सभी `ecat-*` crates workspace में लॉकस्टेप संस्करणों के साथ प्रकाशित होते हैं (वर्तमान 3.0.2), प्रत्येक स्वतंत्र रूप से कंपाइल होता है, उपयोगकर्ता आवश्यकता अनुसार आयात करते हैं। कोर crates न्यूनतम निर्भरताएँ रखते हैं, contrib crates वैकल्पिक एकीकरण प्रदान करते हैं।

### prost (protobuf-rs नहीं) क्यों चुना गया

prost Rust समुदाय में सबसे व्यापक रूप से उपयोग किया जाने वाला protobuf कार्यान्वयन है, कंपाइल-टाइम पर टाइप-सुरक्षित कोड जनरेट करता है, और tonic के साथ गहराई से एकीकृत है।

## डिज़ाइन दस्तावेज़

- [डिज़ाइन विनिर्देश](../../../docs/superpowers/specs/2026-07-29-ecat-framework-design.md)
- [कार्यान्वयन योजना](../../../docs/superpowers/plans/2026-07-29-ecat-framework.md)
- [इकोसिस्टम योजना v1](ecosystem-plan.md)（पूर्ण）
- [इकोसिस्टम योजना v2](ecosystem-plan-v2.md)（पूर्ण）
- [इकोसिस्टम योजना v3](ecosystem-plan-v3.md)（अंतिम मूल्यांकन）
- [API संदर्भ](api.md)
- [ऑडिट रिपोर्ट r5](audit-report-2026-08-01-r5.md)（2026-08-01）
- [डेटाबेस कॉन्फ़िगरेशन ट्यूटोरियल](database-config-tutorial.md)
- [निर्भरता CVE ट्रैकिंग](dependency-cve-tracking.md)
- [TLS प्रमाणपत्र प्रमाणीकरण ट्यूटोरियल](tls-certificate-tutorial.md)
- [कॉन्फ़िगरेशन उदाहरण फ़ाइल](../../../config/databases.example.yaml)

## सहायता

प्रोजेक्ट का समर्थन करने के लिए आपका स्वागत है!

| WeChat Pay | Alipay |
|:---:|:---:|
| <img src="weixinpay.png" width="130" height="130" alt="WeChat Pay"> | <img src="alipay.png" width="130" height="130" alt="Alipay"> |

### वैश्विक स्थानांतरण (बैंक वायर ट्रांसफर)

| आइटम | जानकारी |
|------|------|
| प्राप्तकर्ता का नाम | WANG KEXUN |
| प्राप्तकर्ता खाता संख्या | 881015918251 |
| प्राप्तकर्ता बैंक | ZA Bank Limited |
| SWIFT कोड | AABLHKHHXXX |
| बैंक कोड | 387 |
| बैंक पता | Core F, Cyberport 3, 100 Cyberport Road, Hong Kong |

> **क्रॉस-बॉर्डर रेमिटेंस संवाददाता बैंक (यदि आवश्यक हो)**：यह संवाददाता बैंक (मध्यस्थ बैंक) की जानकारी है, प्राप्तकर्ता बैंक की नहीं, कृपया रेमिट करने वाले बैंक से पूछें कि क्या यह आवश्यक है।
>
> - HKD, RMB और USD के लिए：**Citibank N.A. Hong Kong** (SWIFT：`CITIHKHXXXX`，बैंक कोड：006，शाखा：Hong Kong Branch，शाखा कोड：391，पता：Citibank Tower, Citibank Plaza, 3 Garden Road, Central, Hong Kong)
> - अन्य मुद्राओं के लिए：**THE BANK OF NEW YORK MELLON** (SWIFT：`IRVTUS3NXXX`，पता：240 GREENWICH STREET, NEW YORK, United States)

## लाइसेंस

Apache-2.0
