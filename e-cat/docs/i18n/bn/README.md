<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# Ecat

[简体中文](../../../README.md) | [English](../../../README.en.md) | [日本語](../ja/README.md) | [한국어](../ko/README.md) | [Русский](../ru/README.md) | [Deutsch](../de/README.md) | [Français](../fr/README.md) | [Español](../es/README.md) | [Português](../pt/README.md) | [हिन्दी](../hi/README.md) | [العربية](../ar/README.md) | **বাংলা** | [Bahasa Indonesia](../id/README.md)

Ecat-এর চীনা নাম: একটি বিড়াল (一只猫)

**একটি বিড়াল** হল [go-kratos/kratos](https://github.com/go-kratos/kratos) v3-এর সমতুল্য একটি Rust মাইক্রোসার্ভিস ফ্রেমওয়ার্ক (v3.0.2 · 51 crates)।

API-first উন্নয়ন অভিজ্ঞতা, প্লাগেবল কম্পোনেন্ট আর্কিটেকচার, অভিন্ন HTTP/gRPC মিডলওয়্যার অ্যাবস্ট্রাকশন এবং সম্পূর্ণ CLI টুলচেইন প্রদান করে। Kratos-এর সাথে পরিচিত ডেভেলপাররা নির্বিঘ্নে শুরু করতে পারবেন, পাশাপাশি Rust-এর টাইপ-সেফটি, জিরো-কস্ট অ্যাবস্ট্রাকশন এবং চরম পারফরম্যান্সের সম্পূর্ণ সুবিধা নিতে পারবেন।

<p align="center">
  <img src="e-cat.svg" alt="Ecat প্রকল্পের মাসকট (অ্যানিমেটেড)" width="220" />
</p>

## ডিজাইন আর্কিটেকচার

```
┌──────────────────────────────────────────────────────────────┐
│                         ecat-cli                             │
│        (new │ proto │ run --watch │ build │ upgrade)         │
├──────────────────────────────────────────────────────────────┤
│                     ecat (অ্যাপ্লিকেশন লাইফসাইকেল)            │
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
│                         data স্তর                            │
│     ────────────────────────────────────────────────          │
│     rdbms:   SQLite / PostgreSQL / MySQL / TiDB              │
│     cache:   Redis ✓                                         │
│     config:  remote (Consul KV)                              │
│     registry: consul                                         │
├──────────────────────────────────────────────────────────────┤
│                       ecat-protos                             │
│     (শেয়ার্ড .proto সংজ্ঞা: errors, metadata, ...)           │
└──────────────────────────────────────────────────────────────┘
```

### রিকোয়েস্ট প্রসেসিং ফ্লো

```
ক্লায়েন্ট রিকোয়েস্ট
  │
  ├─ HTTP 0.0.0.0:8000 ──→ axum::Router ──┐
  │                                        │
  └─ gRPC 0.0.0.0:9000 ──→ tonic::Server ─┤
                                      │
                              ┌───────┴───────┐
                              │   Middleware   │
                              │   ──────────   │
                              │ 1. Recovery    │  panic ধরা
                              │ 2. Tracing     │  trace_id ইনজেক্ট
                              │ 3. Logging     │  রিকোয়েস্ট লগ
                              │ 4. Auth        │  প্রমাণীকরণ ও অনুমোদন
                              │ 5. Metrics     │  মেট্রিক সংগ্রহ
│ 6. Security    │  আক্রমণ সনাক্তকরণ
│ 7. CircuitBrk  │  সার্কিট ব্রেকার সুরক্ষা
                              └───────┬───────┘
                                      │
                              ┌───────┴───────┐
                              │    Handler     │  ব্যবহারকারীর ব্যবসায়িক লজিক
                              │ (tower::Service)│
                              └───────┬───────┘
                                      │
                              ┌───────┴───────┐
                              │   Response     │  এনকোডিং ও সিরিয়ালাইজেশন
                              │ JSON/Protobuf  │
                              └───────────────┘
```

## বৈশিষ্ট্য

- **API-first**：Protobuf দিয়ে API, এরর কোড, মেটাডেটা সংজ্ঞায়িত; prost + tonic-build কোড জেনারেশন
- **ডুয়াল-প্রোটোকল সাপোর্ট**：HTTP (axum) এবং gRPC (tonic) একই tower::Layer মিডলওয়্যার সেট শেয়ার করে
- **প্লাগেবল আর্কিটেকচার**：Registry, Config, Logging, Encoding সব trait অ্যাবস্ট্রাকশনের মাধ্যমে; ডিফল্টে প্রোডাকশন-রেডি ইমপ্লিমেন্টেশন
- **মিডলওয়্যার সিস্টেম**：বিল্ট-ইন Recovery, Tracing, Logging, Timeout, RateLimit, Security, CircuitBreaker, MetricsLayer, RetryLayer, ValidateLayer, CORS (cors feature); tower::ServiceBuilder দিয়ে কম্পোজিশন
- **অ্যাপ্লিকেশন লাইফসাইকেল**：Builder প্যাটার্নে App নির্মাণ, একাধিক Server সমান্তরালে চালু, SIGTERM/SIGINT সিগন্যাল হ্যান্ডলিং, start/stop লাইফসাইকেল হুক
- **টাইপ-সেফটি**：protobuf-ভিত্তিক এরর কোড সিস্টেম, কম্পাইল-টাইম HTTP স্ট্যাটাস কোড ম্যাপিং
- **অবজারভেবিলিটি**：tracing + Prometheus + Health এন্ডপয়েন্ট (/health, /ready)
- **আক্রমণ সনাক্তকরণ**：SecurityLayer স্বয়ংক্রিয়ভাবে SQL ইনজেকশন, XSS, SSRF ইত্যাদি আক্রমণ প্যাটার্ন শনাক্ত করে এবং উচ্চ-ঝুঁকির রিকোয়েস্ট ব্লক করে
- **সার্ভিস-টু-সার্ভিস কমিউনিকেশন**：HttpClient সার্ভিস ডিসকভারি ও লোড ব্যালেন্সিং একীভূত; CircuitBreaker সার্কিট ব্রেকার সুরক্ষা
- **প্রমাণীকরণ ও অনুমোদন**：JWT / API Key অথেনটিকেশন মিডলওয়্যার, Claims রিকোয়েস্ট কনটেক্সটে প্রেরণ
- **মেসেজ ও ইভেন্ট**：MessageQueue trait + EventBus লোকাল/রিমোট Pub/Sub
- **ডিস্ট্রিবিউটেড ট্রেসিং**：রিকোয়েস্ট span, trace_id ইনজেকশন/এক্সট্রাকশন
- **gRPC ক্লায়েন্ট**：GrpcClient সার্ভিস ডিসকভারি ও লোড ব্যালেন্সিং একীভূত
- **মাল্টি-প্রোটোকল**：HTTP, gRPC, WebSocket, GraphQL ইউনিফাইড রাউটিং
- **মাল্টি-ডেটাসোর্স**：RDBMS (SQLite/PG/MySQL/TiDB), ক্যাশ (Redis/Memcached), সার্চ (OpenSearch/Elasticsearch), গ্রাফ (Neo4j/NebulaGraph/ArangoDB), টাইম-সিরিজ (InfluxDB/IoTDB/QuestDB/TDengine), ডকুমেন্ট (MongoDB), অবজেক্ট স্টোরেজ (S3/MinIO)

### Kratos কনসেপ্ট ম্যাপিং

| Kratos (Go) | e-cat (Rust) | ব্যাখ্যা |
|-------------|-------------|------|
| `kratos.New()` | `App::builder()` | Builder প্যাটার্ন |
| `http.Handler` | `tower::Service` | Rust ইকোসিস্টেমের স্ট্যান্ডার্ড trait |
| `http.Server` | `axum::Router` | কমিউনিটির মূলধারার HTTP ফ্রেমওয়ার্ক |
| `grpc.Server` | `tonic::transport::Server` | সবচেয়ে পরিণত gRPC ইমপ্লিমেন্টেশন |
| `proto generate` | `prost + tonic-build` | কমিউনিটি স্ট্যান্ডার্ড protobuf |
| `registry.Discovery` | `Registry` trait | প্লাগেবল রেজিস্ট্রি ও ডিসকভারি |
| `config.Source` | `ConfigSource` trait | মাল্টি-সোর্স কনফিগ লোডিং |

## টেকনোলজি স্ট্যাক

| কম্পোনেন্ট | নির্বাচন |
|------|------|
| অ্যাসিংক রানটাইম | **tokio** |
| HTTP | **axum** |
| gRPC | **tonic** |
| Protobuf | **prost + tonic-build** |
| মিডলওয়্যার | **tower::Service / Layer** |
| লগ/ট্রেসিং | **tracing + trace_id propagation** |
| মেট্রিক | **prometheus** |
| সিরিয়ালাইজেশন | **serde + prost** |
| আক্রমণ সনাক্তকরণ | **security-rust** |
| RDBMS | **sqlx** |
| Redis | **redis-rs** |
| JWT | **jsonwebtoken** |
| HTTP ক্লায়েন্ট | **reqwest** |
| CLI | **clap** |

## সমর্থিত ডেটাবেস

| ক্যাটাগরি | ডেটাবেস | Crate | অবস্থা |
|------|--------|-------|------|
| RDBMS | SQLite | `ecat-data-sqlx` | ✅ বাস্তবায়িত |
| RDBMS | PostgreSQL | `ecat-data-sqlx` | ✅ বাস্তবায়িত |
| RDBMS | MySQL | `ecat-data-sqlx` | ✅ বাস্তবায়িত |
| RDBMS | TiDB | `ecat-data-sqlx` | ✅ বাস্তবায়িত |
| ক্যাশ | Redis | `ecat-data-redis` | ✅ বাস্তবায়িত |
| সার্চ | OpenSearch | `ecat-data-opensearch` | ✅ বাস্তবায়িত |
| সার্চ | Elasticsearch | `ecat-data-elasticsearch` | ✅ বাস্তবায়িত |
| ক্যাশ | Memcached | `ecat-data-memcached` | ⚠️ মেমরি-ভিত্তিক (প্রোডাকশন নয়, স্থায়ী ক্যাশের জন্য ব্যবহার করবেন না) |
| OLAP | ClickHouse | `ecat-data-clickhouse` | ✅ বাস্তবায়িত |
| গ্রাফ | Neo4j | `ecat-data-neo4j` | ✅ REST API |
| গ্রাফ | NebulaGraph | `ecat-data-nebulagraph` | ✅ REST API |
| গ্রাফ | ArangoDB | `ecat-data-arangodb` | ✅ REST API |
| টাইম-সিরিজ | InfluxDB | `ecat-data-influxdb` | ✅ HTTP API |
| টাইম-সিরিজ | Apache IoTDB | `ecat-data-iotdb` | ✅ REST API |
| টাইম-সিরিজ | QuestDB | `ecat-data-questdb` | ✅ HTTP API |
| টাইম-সিরিজ | TDengine | `ecat-data-tdengine` | ✅ REST API |
| ডকুমেন্ট | MongoDB | `ecat-data-mongodb` | ✅ নেটিভ ড্রাইভার |
| অবজেক্ট স্টোরেজ | S3 / MinIO | `ecat-data-s3` | ✅ reqwest+rustls |

> সব ডেটা ব্যাকএন্ড ইউনিফাইড trait অ্যাবস্ট্রাকশনের মাধ্যমে (`RdbmsClient` / `Cache` / `SearchClient` / `GraphClient` / `TsdbClient` / `DocumentClient` / `StorageClient`), প্রয়োজন অনুযায়ী সংশ্লিষ্ট contrib crate অন্তর্ভুক্ত করুন। প্রতিটি ব্যাকএন্ড `XxxConfig` স্ট্রাক্ট (`#[derive(Deserialize)]`) প্রদান করে, JSON/YAML কনফিগ ফাইল থেকে সংযোগ তথ্য লোড করা যায়।

> **কনস্ট্রাক্টর নামকরণ কনভেনশন**：মেসেজ কিউ crates (`ecat-mq-*`) এর প্রধান কনস্ট্রাক্টর একই `connect` (যেমন `KafkaMq::connect(brokers)`, `MqttMq::connect(url)`), সাথে `from_config` কনফিগ থেকে লোড করার জন্য; ডেটা ব্যাকএন্ড crates (`ecat-data-*`) এর বেশিরভাগ প্রধান কনস্ট্রাক্টর `new`, ব্যতিক্রম: `ecat-data-redis` / `ecat-data-sqlx` `connect` অনুসরণ করে, `ecat-data-mongodb` / `ecat-data-s3` শুধুমাত্র `from_config` প্রদান করে। এটি বিদ্যমান কনভেনশন, বাধ্যতামূলকভাবে একীভূত নয় (ব্রেকিং পরিবর্তন এড়াতে); 3.0 উইন্ডোতে একীকরণ মূল্যায়ন করা যেতে পারে।

### ডেটাবেস কনফিগ উদাহরণ

প্রতিটি ডেটা ব্যাকএন্ড `XxxConfig` স্ট্রাক্ট এবং `from_config()` মেথড প্রদান করে, সংযোগ তথ্য কোড থেকে কনফিগ ফাইলে বিচ্ছিন্ন করে:

```rust
use ecat_data_redis::{RedisCache, RedisConfig};
use ecat_data_sqlx::{SqlxClient, SqlxConfig};
use ecat_data_clickhouse::{ClickhouseClient, ClickhouseConfig};

// কনফিগ ফাইল থেকে লোড (JSON বা YAML)
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

**কনফিগ ফিল্ড রেফারেন্স**:

| ব্যাকএন্ড | Config | ফিল্ড | উদাহরণ মান |
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
| Memcached | `MemcachedConfig` | `username`?, `password`? (রিজার্ভড ফিল্ড) | — |
| TDengine | `TdengineConfig` | `base_url`, `username`, `password`, `database`? | `http://localhost:6041` |
| MongoDB | `MongoConfig` | `url`, `database`, `tls`? | `mongodb://localhost:27017`, `app` |
| S3 | `S3Config` | `endpoint`, `region`, `access_key`, `secret_key`, `tls`? | `http://localhost:9000`, `us-east-1` |

> সব ব্যাকএন্ড Config ঐচ্ছিক `tls` ফিল্ড (`TlsClientConfig`) সমর্থন করে, TLS ক্লায়েন্ট সার্টিফিকেট অথেনটিকেশন কনফিগ করতে। বিস্তারিত দেখুন [ডেটাবেস কনফিগ টিউটোরিয়াল](database-config-tutorial.md)।

## প্রকল্প কাঠামো

```
e-cat/
├── ecat/                       # কোর: App লাইফসাইকেল
├── ecat-transport/             # ট্রান্সপোর্ট অ্যাবস্ট্রাকশন (Server trait)
├── ecat-transport-http/        # axum ইমপ্লিমেন্টেশন
├── ecat-transport-grpc/        # tonic ইমপ্লিমেন্টেশন
├── ecat-middleware/            # tower::Layer মিডলওয়্যার
├── ecat-protos/                # Protobuf সংজ্ঞা
├── ecat-errors/                # এরর কোড সিস্টেম
├── ecat-metadata/              # মেটাডেটা প্রেরণ
├── ecat-encoding/              # সিরিয়ালাইজেশন অ্যাবস্ট্রাকশন
├── ecat-logging/               # tracing ইন্টিগ্রেশন
├── ecat-registry/              # সার্ভিস রেজিস্ট্রি ও ডিসকভারি
├── ecat-config/                # কনফিগ ম্যানেজমেন্ট
├── ecat-metrics/               # Prometheus ইন্টিগ্রেশন
├── ecat-data/                  # ডেটা অ্যাক্সেস trait
├── ecat-security/              # আক্রমণ সনাক্তকরণ (security-rust)
├── ecat-cli/                   # CLI টুল
├── ecat-health/                # হেলথ চেক (/health /ready)
├── ecat-auth/                  # অথেনটিকেশন মিডলওয়্যার (JWT / API Key)
├── ecat-client/                # সার্ভিস-টু-সার্ভিস HTTP ক্লায়েন্ট
├── ecat-circuit-breaker/       # সার্কিট ব্রেকার (Tower Layer)
├── ecat-registry-consul/       # Consul সার্ভিস রেজিস্ট্রি
├── ecat-config-remote/         # Consul KV রিমোট কনফিগ
├── ecat-data-redis/            # Redis ক্যাশ ইমপ্লিমেন্টেশন
├── ecat-mq/                    # মেসেজ কিউ অ্যাবস্ট্রাকশন
├── ecat-events/                # ইভেন্ট বাস (লোকাল + রিমোট)
├── ecat-testing/               # ইন্টিগ্রেশন টেস্ট টুল
├── ecat-openapi/               # OpenAPI spec জেনারেশন
├── ecat-bench/                 # পারফরম্যান্স বেঞ্চমার্ক
├── ecat-tracing/               # ডিস্ট্রিবিউটেড ট্রেসিং (trace_id ইনজেকশন/এক্সট্রাকশন)
├── ecat-registry-etcd/         # etcd সার্ভিস রেজিস্ট্রি
├── ecat-mq-kafka/              # Kafka মেসেজ কিউ অ্যাডাপ্টার
├── ecat-data-opensearch/       # OpenSearch সার্চ ব্যাকএন্ড
├── ecat-data-influxdb/         # InfluxDB টাইম-সিরিজ ব্যাকএন্ড
├── ecat-graphql/               # GraphQL endpoint
├── ecat-data-elasticsearch/    # Elasticsearch সার্চ ব্যাকএন্ড
├── ecat-data-clickhouse/       # ClickHouse OLAP ব্যাকএন্ড
├── ecat-data-sqlx/             # RDBMS ব্যাকএন্ড (SQLite/PG/MySQL/TiDB)
├── ecat-data-memcached/        # Memcached ক্যাশ ব্যাকএন্ড (মেমরি-ভিত্তিক)
├── ecat-data-neo4j/            # Neo4j গ্রাফ ব্যাকএন্ড
├── ecat-data-nebulagraph/      # NebulaGraph গ্রাফ ব্যাকএন্ড
├── ecat-data-arangodb/         # ArangoDB গ্রাফ ব্যাকএন্ড
├── ecat-data-iotdb/            # IoTDB টাইম-সিরিজ ব্যাকএন্ড
├── ecat-data-questdb/          # QuestDB টাইম-সিরিজ ব্যাকএন্ড
├── ecat-transport-ws/          # WebSocket transport
├── ecat-versioning/            # API ভার্সন রাউটিং
├── ecat-tls/                   # TLS সার্টিফিকেট কনফিগ ও অটো-জেনারেশন
├── ecat-deploy/                # Docker / K8s / Helm / CI/CD
├── ecat-lock/                  # ডিস্ট্রিবিউটেড লক অ্যাবস্ট্রাকশন (Redis ইমপ্লিমেন্টেশন)
├── ecat-scheduler/             # tokio টাইমড টাস্ক শিডিউলিং
├── ecat-tracing-otlp/          # OpenTelemetry OTLP ট্রেসিং এক্সপোর্ট
├── ecat-data-tdengine/         # TDengine টাইম-সিরিজ ব্যাকএন্ড
├── ecat-data-mongodb/          # MongoDB ডকুমেন্ট ব্যাকএন্ড
├── ecat-data-s3/               # S3 / MinIO অবজেক্ট স্টোরেজ ব্যাকএন্ড
├── ecat-mq-rabbitmq/           # RabbitMQ মেসেজ ব্যাকএন্ড
├── ecat-mq-mqtt/               # MQTT মেসেজ ব্যাকএন্ড
├── ecat-mq-nats/               # NATS মেসেজ ব্যাকএন্ড
├── config/                     # কনফিগ উদাহরণ ফাইল
├── docs/                       # ডিজাইন ডকুমেন্ট ও ইকোসিস্টেম পরিকল্পনা
└── examples/                   # উদাহরণ প্রকল্প
```

## দ্রুত শুরু

### পূর্বশর্ত

- Rust 1.85+ (stable টুলচেইন, edition 2024 প্রয়োজন)
- [protoc](https://github.com/protocolbuffers/protobuf) (Protocol Buffers কম্পাইলার)

### CLI ইনস্টল

```bash
cargo install ecat-cli
```

### সার্ভিস তৈরি

```bash
# স্ক্যাফোল্ড দিয়ে প্রকল্প জেনারেট
ecat new helloworld
cd helloworld

# proto সংজ্ঞা যোগ করুন
ecat proto add proto/service.proto

# ক্লায়েন্ট ও সার্ভার কোড জেনারেট (tonic-build build.rs, Cargo.toml ডিপেন্ডেন্সি অটো-সম্পূর্ণ)
ecat proto client proto/service.proto
ecat proto server proto/service.proto -t internal/service

# ডেভেলপমেন্ট মোডে চালান
ecat run

# src/ পরিবর্তন দেখে অটো-রিস্টার্ট
ecat run --watch

# সব ecat-* ডিপেন্ডেন্সি আপডেট
ecat upgrade
```

`http://localhost:8000/helloworld/ecat` অ্যাক্সেস করুন।

### কোড উদাহরণ

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

    app.run().await?; // SIGTERM/SIGINT পর্যন্ত ব্লক করে
    Ok(())
}
```

### অ্যাগ্রিগেট crate (ecat)

`ecat` feature-gated re-export এন্ট্রি পয়েন্ট প্রদান করে — শুধুমাত্র প্রয়োজনীয় কম্পোনেন্ট সক্ষম করুন:

```rust
use ecat::transport_http::HttpServer;   // feature "http" (ডিফল্ট)
use ecat::middleware::RecoveryLayer;     // feature "middleware"
use ecat::auth::JwtAuthLayer;            // feature "auth"
use ecat::data::redis::RedisCache;       // feature "redis"
```

ডিফল্ট features = `http+grpc`; `--no-default-features --features <কম্পোনেন্ট>` দিয়ে ডিপেন্ডেন্সি ট্রি কমানো যায়। সম্পূর্ণ feature তালিকা: `http` `grpc` `middleware` `auth` `client` `events` `metrics` `tracing` `circuit-breaker` `consul` `remote` `redis`।

### মিডলওয়্যার

```rust
use tower::ServiceBuilder;
use ecat_middleware::{RecoveryLayer, TracingLayer, LoggingLayer, TimeoutLayer};
use ecat_circuit_breaker::CircuitBreakerLayer;
use ecat_security::SecurityLayer;
use ecat_auth::JwtAuthLayer;
use std::time::Duration;

// JWT সিক্রেট ≥32 বাইট হতে হবে; চেইনযোগ্য ভাবে iss/aud ক্লেইম যাচাই (ঐচ্ছিক, ডিফল্টে যাচাই নয়):
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

> নোট: `ecat_middleware::TracingLayer` trace_id ইনজেক্ট করে না; রিকোয়েস্ট-লেভেল trace_id ইনজেকশনের জন্য `ecat_tracing::TracingLayer::new()` ব্যবহার করুন।

```rust
// মেট্রিক: রিকোয়েস্ট কাউন্ট ও ল্যাটেন্সি গ্লোবাল registry-তে রেকর্ড (/metrics এন্ডপয়েন্টের সাথে শেয়ার্ড)
use ecat_metrics::MetricsLayer;
let app = Router::new().route("/hello", get(hello)).layer(MetricsLayer::new());
// মেট্রিক নাম: ecat_http_requests_total / ecat_http_request_duration_seconds
// (লেবেল method/path/status)। পাথে ID-এর মতো উচ্চ-কার্ডিনালিটি হলে
// MetricsLayer::new().with_path_fn(...) দিয়ে নরমালাইজ করুন, মেট্রিক কার্ডিনালিটি বিস্ফোরণ এড়াতে।

// রিট্রাই: এক্সপোনেনশিয়াল ব্যাকঅফ; ⚠️ শুধুমাত্র আইডেম্পোটেন্ট রিকোয়েস্টের জন্য নিরাপদ (GET/HEAD/PUT/DELETE)
use ecat_middleware::RetryLayer;
let retry = RetryLayer::new(3, Duration::from_secs(1), Duration::from_secs(30)); // প্রথমসহ মোট 3 বার চেষ্টা
// কাস্টম রিট্রাই রুল: RetryLayer::new(3, ...).with_rule(MyRule)  // স্ট্যাটাস কোড/রেসপন্স কনটেন্ট অনুযায়ী সিদ্ধান্ত

// ভ্যালিডেশন: রাউটের আগে header/প্যারামিটার যাচাই, ব্যর্থ হলে শর্ট-সার্কিট JSON এরর (ডিফল্ট 400, with_status দিয়ে 422 ইত্যাদি)
use ecat_middleware::{ValidateLayer, ValidateError};
let validate = ValidateLayer::from_fn(|req: &http::Request<axum::body::Body>| {
    if req.headers().contains_key("x-api-key") {
        Ok(())
    } else {
        Err(ValidateError::new("missing x-api-key").with_status(422))
    }
});

// CORS: ecat-middleware-এ "cors" feature সক্ষম করতে হবে
use ecat_middleware::{CorsLayer, AllowOrigin};
let cors = CorsLayer::new().allow_origin(AllowOrigin::any());
```

### এরর হ্যান্ডলিং

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

## ইমপ্লিমেন্টেশন ফেজ

| ফেজ | অবস্থা | বিষয়বস্তু |
|------|------|------|
| Phase 1 | ✅ সম্পন্ন | প্রকল্প স্কেলেটন, protos, errors, metadata, encoding, logging |
| Phase 2 | ✅ সম্পন্ন | Transport স্তর (HTTP + gRPC) |
| Phase 3 | ✅ সম্পন্ন | Middleware সিস্টেম (Recovery/Tracing/Logging/Timeout) |
| Phase 4 | ✅ সম্পন্ন | App লাইফসাইকেল ম্যানেজমেন্ট |
| Phase 5 | ✅ সম্পন্ন | Registry, Config, Metrics |
| Phase 5.5 | ✅ সম্পন্ন | Data অ্যাক্সেস স্তর (traits + sqlx ব্যাকএন্ড) |
| Phase 6 | ✅ সম্পন্ন | CLI টুলচেইন (new/proto/run/build) |
| Phase 7 | ✅ সম্পন্ন | README, উদাহরণ (helloworld), ডিজাইন ডকুমেন্ট |
| Phase 8 | ✅ সম্পন্ন | আক্রমণ সনাক্তকরণ ইন্টিগ্রেশন (security-rust, ecat-security) |
| Phase 9 | ✅ সম্পন্ন | ইকোসিস্টেম পর্ব 1 (health / client / circuit-breaker / auth / registry-consul) |
| Phase 10 | ✅ সম্পন্ন | ইকোসিস্টেম পর্ব 2 (redis / mq / events / config-remote) |
| Phase 11 | ✅ সম্পন্ন | ইকোসিস্টেম পর্ব 3 (testing / deploy / bench / openapi) |
| Phase 12 | ✅ সম্পন্ন | কমিউনিকেশন ও সিকিউরিটি শক্তিশালীকরণ (gRPC ক্লায়েন্ট / OAuth2 / mTLS / ডিস্ট্রিবিউটেড ট্রেসিং) |
| Phase 13 | ✅ সম্পন্ন | ডেটা ব্যাকএন্ড সম্পূর্ণকরণ (etcd / Kafka / OpenSearch / InfluxDB) |
| Phase 14 | ✅ সম্পন্ন | অপারেশন ও অভিজ্ঞতা (WebSocket / API ভার্সন ম্যানেজমেন্ট / Helm / CI/CD) |
| Phase 15 | ✅ সম্পন্ন | ইকোসিস্টেম এক্সটেনশন v2 (আসল Kafka / RabbitMQ / MQTT / NATS / MongoDB / S3 / TDengine / OTLP / ডিস্ট্রিবিউটেড লক / শিডিউলিং / CLI watch+upgrade) |
| Phase 16 | ✅ সম্পন্ন | রক্ষণাবেক্ষণ শক্তিশালীকরণ v2.4 (M1 MetricsLayer / M2 RetryLayer / M3 ValidateLayer / M4 CORS / U1 অ্যাগ্রিগেট crate ecat / U2 examples / OAuth2 token hash / CVE ট্র্যাকিং) |

## পরিচিত সীমাবদ্ধতা

- **GraphQL পার্সিং (ecat-graphql)**：ফিল্ড প্যারামিটার ও নেস্টেড selection সমর্থন করে (`query_field`/`mutation_field` রিচ resolver-এ `args`/`variables`/`selection` অ্যাক্সেস); এখনও alias, fragment ও মাল্টি-টপ-লেভেল ফিল্ড সমর্থন করে না, দয়া করে এটিকে সাধারণ GraphQL এন্ডপয়েন্ট হিসেবে প্রকাশ করবেন না।
- **OAuth2 ইন্ট্রোস্পেকশন ক্যাশ (ecat-auth)**：ক্যাশ key হল token-এর SHA-256 hash (token প্লেইনটেক্সট সংরক্ষিত নয়); ক্যাশ মান হোয়াইটলিস্ট ফিল্টারের মাধ্যমে (ডিফল্টে sub/exp/iat/role + extra-র iss/aud/scope/roles, `cache_claims_whitelist` কনফিগারেবল; miss হলে পূর্ণ claims ফেরত দেওয়া হয়, শুধুমাত্র ক্যাশ মান ফিল্টার হয়); TTL মেয়াদোত্তীর্ণ এন্ট্রি লেখার সময় সক্রিয়ভাবে পরিষ্কার হয় (ডিফল্ট TTL 300s)।
- **Kafka offset (ecat-mq-kafka)**：ডিফল্ট `enable.auto.commit=false` এবং ম্যানুয়াল commit নেই — প্রসেস রিস্টার্টে পার্টিশনের শেষ (latest) থেকে পুনরায় পড়া হয়, ডাউনটাইমের সময় উৎপন্ন মেসেজ স্কিপ হয়; at-least-once সেমান্টিক্সের জন্য `auto_commit=true` স্পষ্টভাবে কনফিগ করতে হবে (রিস্টার্টে সাম্প্রতিক commit পয়েন্ট থেকে চলতে থাকে)।

## ডিজাইন লক্ষ্য

| # | লক্ষ্য | ব্যাখ্যা |
|---|------|------|
| 1 | **Kratos অ্যালাইনমেন্ট** | Kratos-এর API-first, প্লাগেবল, ইউনিফাইড অ্যাবস্ট্রাকশন দর্শন বজায় রাখা |
| 2 | **Rust ইডিওম্যাটিক** | tower::Service, trait জেনেরিক্স, জিরো-কস্ট অ্যাবস্ট্রাকশন পুনঃব্যবহার; "Go in Rust" নয় |
| 3 | **টাইপ-সেফটি** | কম্পাইল-টাইমে এরর ধরা, Protobuf সংজ্ঞা সম্পূর্ণরূপে টাইপ-সেফ |
| 4 | **প্লাগেবল** | Registry, Config, Logging, Encoding সব trait অ্যাবস্ট্রাকশনের মাধ্যমে |
| 5 | **সম্পূর্ণ টুলচেইন** | CLI প্রকল্প স্ক্যাফোল্ডিং, proto কোড জেনারেশন, ডেভেলপমেন্ট রান সমর্থন করে |
| 6 | **পারফরম্যান্স-প্রথম** | জিরো-কস্ট অ্যাবস্ট্রাকশন + অ্যাসিংক রানটাইম |
| 7 | **অবজারভেবল** | tracing + Prometheus আউট-অফ-বক্স |
| 8 | **সম্পূর্ণ ইকোসিস্টেম** | ক্লায়েন্ট, সার্কিট ব্রেকার, অথেনটিকেশন, হেলথ চেক, রেজিস্ট্রি ব্যাকএন্ড |

## প্রযুক্তিগত ব্যাখ্যা

### কেন tower::Service

[`tower::Service`](https://docs.rs/tower/latest/tower/trait.Service.html) হল Rust অ্যাসিংক ইকোসিস্টেমের `http.Handler` সমতুল্য। axum এবং tonic দুটোই tower-এর উপর নির্মিত, তাই e-cat-এর কাস্টম মিডলওয়্যার trait দরকার হয় না — সরাসরি tower::Layer ইমপ্লিমেন্টেশন প্রদান করলেই Kratos মিডলওয়্যারের সমান ফলাফল পাওয়া যায়, জিরো অ্যাডাপ্টার ওভারহেডে।

### কেন Cargo Workspace

Kratos-এর মডুলার ডিজাইনের সাথে সামঞ্জস্যপূর্ণ। সব `ecat-*` crate workspace লক-স্টেপ ভার্সনে প্রকাশিত হয় (বর্তমান 3.0.2), প্রতিটি স্বাধীনভাবে কম্পাইল হয়, ব্যবহারকারী প্রয়োজন অনুযায়ী অন্তর্ভুক্ত করে। কোর crates ন্যূনতম ডিপেন্ডেন্সি রাখে, contrib crates ঐচ্ছিক ইন্টিগ্রেশন প্রদান করে।

### কেন prost (protobuf-rs নয়)

prost হল Rust কমিউনিটিতে সবচেয়ে ব্যাপকভাবে ব্যবহৃত protobuf ইমপ্লিমেন্টেশন, কম্পাইল-টাইমে টাইপ-সেফ কোড জেনারেট করে এবং tonic-এর সাথে গভীরভাবে একীভূত।

## ডিজাইন ডকুমেন্ট

- [ডিজাইন স্পেক](../../../docs/superpowers/specs/2026-07-29-ecat-framework-design.md)
- [ইমপ্লিমেন্টেশন প্ল্যান](../../../docs/superpowers/plans/2026-07-29-ecat-framework.md)
- [ইকোসিস্টেম প্ল্যান v1](ecosystem-plan.md)（সম্পন্ন）
- [ইকোসিস্টেম প্ল্যান v2](ecosystem-plan-v2.md)（সম্পন্ন）
- [ইকোসিস্টেম প্ল্যান v3](ecosystem-plan-v3.md)（চূড়ান্ত মূল্যায়ন）
- [API রেফারেন্স](api.md)
- [অডিট রিপোর্ট r5](audit-report-2026-08-01-r5.md)（2026-08-01）
- [ডেটাবেস কনফিগ টিউটোরিয়াল](database-config-tutorial.md)
- [ডিপেন্ডেন্সি CVE ট্র্যাকিং](dependency-cve-tracking.md)
- [TLS সার্টিফিকেট অথেনটিকেশন টিউটোরিয়াল](tls-certificate-tutorial.md)
- [কনফিগ উদাহরণ ফাইল](../../../config/databases.example.yaml)

## সাপোর্ট

এই প্রকল্পকে সাপোর্ট করতে স্বাগতম!

| WeChat Pay | Alipay |
|:---:|:---:|
| <img src="weixinpay.png" width="130" height="130" alt="WeChat Pay"> | <img src="alipay.png" width="130" height="130" alt="Alipay"> |

### গ্লোবাল ট্রান্সফার (ব্যাংক ওয়্যার ট্রান্সফার)

| আইটেম | তথ্য |
|------|------|
| প্রাপকের নাম | WANG KEXUN |
| প্রাপকের অ্যাকাউন্ট নম্বর | 881015918251 |
| প্রাপকের ব্যাংক | ZA Bank Limited |
| SWIFT কোড | AABLHKHHXXX |
| ব্যাংক কোড | 387 |
| ব্যাংকের ঠিকানা | Core F, Cyberport 3, 100 Cyberport Road, Hong Kong |

> **ক্রস-বর্ডার রেমিট্যান্সের জন্য করেসপনডেন্ট ব্যাংক (যদি প্রয়োজন হয়)**：এটি করেসপনডেন্ট ব্যাংক (মধ্যস্থ ব্যাংক) এর তথ্য, প্রাপকের ব্যাংকের তথ্য নয়; অনুগ্রহ করে পাঠানোর ব্যাংকে জিজ্ঞাসা করুন এটি প্রয়োজন কিনা।
>
> - হংকং ডলার, রেনমিনবি ও মার্কিন ডলার প্রেরণ: **Citibank N.A. Hong Kong**（SWIFT：`CITIHKHXXXX`，ব্যাংক কোড：006，শাখা：Hong Kong Branch，শাখা কোড：391，ঠিকানা：Citibank Tower, Citibank Plaza, 3 Garden Road, Central, Hong Kong）
> - অন্যান্য মুদ্রা প্রেরণ: **THE BANK OF NEW YORK MELLON**（SWIFT：`IRVTUS3NXXX`，ঠিকানা：240 GREENWICH STREET, NEW YORK, United States）

## লাইসেন্স

Apache-2.0
